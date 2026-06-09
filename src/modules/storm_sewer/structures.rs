// Interactive storm-sewer drafting commands.
//
// Both commands collect typed values FIRST (the command line stays focused),
// then take the viewport interaction LAST and commit from it. This mirrors the
// OFFSET command's flow and avoids losing command-line focus after a click
// (which would otherwise route Enter to on_enter and cancel the command).
//
// PlaceStructure: enter invert/rim/area/C, then click the location.
// PlacePipe: enter diameter/n, then click the START and END structures.

use acadrust::types::Vector3;
use acadrust::{Circle, EntityType, Handle, Line};
use glam::Vec3;

use stormsewer::network::NodeKind;

use super::data;
use crate::command::{CadCommand, CmdResult};

fn parse_num(text: &str) -> Option<f64> {
    text.trim().replace(',', ".").parse::<f64>().ok()
}

// ── Structure placement ─────────────────────────────────────────────────────

enum SStep {
    Invert,
    Rim,
    Area,
    C,
    Point,
}

pub struct PlaceStructure {
    kind: NodeKind,
    radius: f64,
    invert: f64,
    rim: f64,
    area: f64,
    c: f64,
    step: SStep,
}

impl PlaceStructure {
    pub fn inlet() -> Self {
        Self::new(NodeKind::Inlet, 3.0)
    }
    pub fn junction() -> Self {
        Self::new(NodeKind::Junction, 4.0)
    }
    pub fn outfall() -> Self {
        Self::new(NodeKind::Outfall, 6.0)
    }
    fn new(kind: NodeKind, radius: f64) -> Self {
        Self { kind, radius, invert: 100.0, rim: 105.0, area: 1.0, c: 0.70, step: SStep::Invert }
    }
    fn commit(&self, x: f64, y: f64) -> CmdResult {
        let circ = Circle { center: Vector3::new(x, y, 0.0), radius: self.radius, ..Default::default() };
        let mut ent = EntityType::Circle(circ);
        let (area, c) = if self.kind == NodeKind::Outfall { (0.0, 0.0) } else { (self.area, self.c) };
        ent.common_mut()
            .extended_data
            .add_record(data::structure_xdata(self.kind, self.invert, self.rim, area, c));
        CmdResult::CommitAndExit(ent)
    }
}

impl CadCommand for PlaceStructure {
    fn name(&self) -> &'static str {
        "SS_STRUCTURE"
    }
    fn prompt(&self) -> String {
        match self.step {
            SStep::Invert => format!("Invert elevation <{:.2}>:", self.invert),
            SStep::Rim => format!("Rim elevation <{:.2}>:", self.rim),
            SStep::Area => format!("Drainage area, ac <{:.2}>:", self.area),
            SStep::C => format!("Runoff coefficient C <{:.2}>:", self.c),
            SStep::Point => format!("Storm {}: pick location:", data::kind_str(self.kind)),
        }
    }
    fn wants_text_input(&self) -> bool {
        !matches!(self.step, SStep::Point)
    }
    fn on_text_input(&mut self, text: &str) -> Option<CmdResult> {
        let v = parse_num(text);
        match self.step {
            SStep::Invert => {
                if let Some(x) = v {
                    self.invert = x;
                }
                self.step = SStep::Rim;
            }
            SStep::Rim => {
                if let Some(x) = v {
                    self.rim = x;
                }
                self.step = if self.kind == NodeKind::Outfall { SStep::Point } else { SStep::Area };
            }
            SStep::Area => {
                if let Some(x) = v {
                    self.area = x;
                }
                self.step = SStep::C;
            }
            SStep::C => {
                if let Some(x) = v {
                    self.c = x;
                }
                self.step = SStep::Point;
            }
            SStep::Point => {}
        }
        None // never commit from text; the location click commits
    }
    fn on_point(&mut self, pt: Vec3) -> CmdResult {
        if let SStep::Point = self.step {
            self.commit(pt.x as f64, pt.y as f64)
        } else {
            CmdResult::NeedPoint
        }
    }
    fn on_enter(&mut self) -> CmdResult {
        // Only reached at the Point step (text steps route empty Enter to
        // on_text_input). Keep waiting for the click rather than cancelling.
        CmdResult::NeedPoint
    }
}

// ── Pipe placement ──────────────────────────────────────────────────────────

enum PStep {
    Diameter,
    N,
    PickStart,
    PickEnd,
}

pub struct PlacePipe {
    step: PStep,
    diameter: f64,
    n: f64,
    start_handle: Option<Handle>,
    start_xy: (f64, f64),
}

impl PlacePipe {
    pub fn new() -> Self {
        Self { step: PStep::Diameter, diameter: 1.25, n: 0.013, start_handle: None, start_xy: (0.0, 0.0) }
    }
    fn commit(&self, end_handle: Handle, ex: f64, ey: f64) -> CmdResult {
        let line = Line::from_points(
            Vector3::new(self.start_xy.0, self.start_xy.1, 0.0),
            Vector3::new(ex, ey, 0.0),
        );
        let mut ent = EntityType::Line(line);
        let from = self.start_handle.unwrap_or(Handle::new(0));
        ent.common_mut().extended_data.add_record(data::pipe_xdata(self.diameter, self.n, from, end_handle));
        CmdResult::CommitAndExit(ent)
    }
}

impl Default for PlacePipe {
    fn default() -> Self {
        Self::new()
    }
}

impl CadCommand for PlacePipe {
    fn name(&self) -> &'static str {
        "SS_PIPE"
    }
    fn prompt(&self) -> String {
        match self.step {
            PStep::Diameter => format!("Pipe diameter, ft <{:.2}>:", self.diameter),
            PStep::N => format!("Manning n <{:.3}>:", self.n),
            PStep::PickStart => "Pipe: pick START structure:".into(),
            PStep::PickEnd => "Pipe: pick END structure:".into(),
        }
    }
    fn wants_text_input(&self) -> bool {
        matches!(self.step, PStep::Diameter | PStep::N)
    }
    fn on_text_input(&mut self, text: &str) -> Option<CmdResult> {
        let v = parse_num(text);
        match self.step {
            PStep::Diameter => {
                if let Some(x) = v {
                    self.diameter = x;
                }
                self.step = PStep::N;
            }
            PStep::N => {
                if let Some(x) = v {
                    self.n = x;
                }
                self.step = PStep::PickStart;
            }
            _ => {}
        }
        None
    }
    fn needs_entity_pick(&self) -> bool {
        matches!(self.step, PStep::PickStart | PStep::PickEnd)
    }
    fn on_entity_pick(&mut self, handle: Handle, pt: Vec3) -> CmdResult {
        match self.step {
            PStep::PickStart => {
                self.start_handle = Some(handle);
                self.start_xy = (pt.x as f64, pt.y as f64);
                self.step = PStep::PickEnd;
                CmdResult::NeedPoint
            }
            PStep::PickEnd => self.commit(handle, pt.x as f64, pt.y as f64),
            _ => CmdResult::NeedPoint,
        }
    }
    fn on_point(&mut self, _pt: Vec3) -> CmdResult {
        CmdResult::NeedPoint
    }
    fn on_enter(&mut self) -> CmdResult {
        CmdResult::NeedPoint
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn structure_prompts_then_commits_tagged_circle_on_click() {
        let mut cmd = PlaceStructure::inlet();
        assert!(cmd.wants_text_input(), "should start by asking for invert");
        assert!(cmd.on_text_input("104").is_none()); // invert -> rim
        assert!(cmd.on_text_input("110").is_none()); // rim -> area
        assert!(cmd.on_text_input("2.0").is_none()); // area -> C
        assert!(cmd.on_text_input("0.8").is_none()); // C -> Point
        assert!(!cmd.wants_text_input(), "should now wait for the location click");
        match cmd.on_point(Vec3::new(10.0, 20.0, 0.0)) {
            CmdResult::CommitAndExit(EntityType::Circle(c)) => {
                assert_eq!(c.center.x, 10.0);
                let e = EntityType::Circle(c);
                assert!(e.common().extended_data.get_record(data::APP_STRUCT).is_some());
            }
            _ => panic!("expected CommitAndExit(Circle) with XDATA"),
        }
    }

    #[test]
    fn outfall_skips_area_and_c() {
        let mut cmd = PlaceStructure::outfall();
        assert!(cmd.on_text_input("100").is_none()); // invert -> rim
        assert!(cmd.on_text_input("105").is_none()); // rim -> Point (no area/C)
        assert!(!cmd.wants_text_input());
        assert!(matches!(cmd.on_point(Vec3::ZERO), CmdResult::CommitAndExit(_)));
    }

    #[test]
    fn pipe_enters_size_then_connects_two_structures() {
        let mut cmd = PlacePipe::new();
        assert!(cmd.wants_text_input(), "should ask diameter first");
        assert!(cmd.on_text_input("1.5").is_none()); // diameter -> n
        assert!(cmd.on_text_input("0.013").is_none()); // n -> PickStart
        assert!(cmd.needs_entity_pick(), "now picking structures");
        // pick start at (0,0)
        assert!(matches!(cmd.on_entity_pick(Handle::new(1), Vec3::new(0.0, 0.0, 0.0)), CmdResult::NeedPoint));
        // pick end at (100,0) -> commits a line carrying connectivity XDATA
        match cmd.on_entity_pick(Handle::new(2), Vec3::new(100.0, 0.0, 0.0)) {
            CmdResult::CommitAndExit(EntityType::Line(l)) => {
                assert_eq!(l.start.x, 0.0);
                assert_eq!(l.end.x, 100.0);
                let e = EntityType::Line(l);
                assert!(e.common().extended_data.get_record(data::APP_PIPE).is_some());
            }
            _ => panic!("expected CommitAndExit(Line) with XDATA"),
        }
    }
}
