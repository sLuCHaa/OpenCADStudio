//! Dimension Style Manager window — fills the entire OS window.

use crate::app::{DsField, Message};
use iced::widget::{button, checkbox, column, container, row, scrollable, text, text_input, Space};
use iced::{Background, Border, Color, Element, Fill, Theme};

const TB: Color = Color {
    r: 0.13,
    g: 0.13,
    b: 0.13,
    a: 1.0,
};
const BG: Color = Color {
    r: 0.15,
    g: 0.15,
    b: 0.15,
    a: 1.0,
};
const BORDER: Color = Color {
    r: 0.35,
    g: 0.35,
    b: 0.35,
    a: 1.0,
};
const TEXT: Color = Color {
    r: 0.88,
    g: 0.88,
    b: 0.88,
    a: 1.0,
};
const DIM: Color = Color {
    r: 0.55,
    g: 0.55,
    b: 0.55,
    a: 1.0,
};
const ACCENT: Color = Color {
    r: 0.25,
    g: 0.50,
    b: 0.85,
    a: 1.0,
};
const ACTIVE: Color = Color {
    r: 0.20,
    g: 0.40,
    b: 0.70,
    a: 1.0,
};
const FIELD: Color = Color {
    r: 0.10,
    g: 0.10,
    b: 0.10,
    a: 1.0,
};
const LIST: Color = Color {
    r: 0.12,
    g: 0.12,
    b: 0.12,
    a: 1.0,
};

/// All DimStyle field values needed by the view.
pub struct DimStyleValues<'a> {
    pub dimdle: &'a str,
    pub dimdli: &'a str,
    pub dimgap: &'a str,
    pub dimexe: &'a str,
    pub dimexo: &'a str,
    pub dimsd1: bool,
    pub dimsd2: bool,
    pub dimse1: bool,
    pub dimse2: bool,
    pub dimasz: &'a str,
    pub dimcen: &'a str,
    pub dimtsz: &'a str,
    pub dimtxt: &'a str,
    pub dimtxsty: &'a str,
    pub dimtad: &'a str,
    pub dimtih: bool,
    pub dimtoh: bool,
    pub dimscale: &'a str,
    pub dimlfac: &'a str,
    pub dimlunit: &'a str,
    pub dimdec: &'a str,
    pub dimpost: &'a str,
    pub dimtol: bool,
    pub dimlim: bool,
    pub dimtp: &'a str,
    pub dimtm: &'a str,
    pub dimtdec: &'a str,
    pub dimtfac: &'a str,
    pub annotative: bool,
    pub dimclrd: &'a str,
    pub dimlwd: &'a str,
    pub dimclre: &'a str,
    pub dimlwe: &'a str,
    pub dimfxl: &'a str,
    pub dimfxlon: bool,
    pub dimsah: bool,
    pub dimarcsym: &'a str,
    pub dimjogang: &'a str,
    pub dimclrt: &'a str,
    pub dimjust: &'a str,
    pub dimtvp: &'a str,
    pub dimtfill: &'a str,
    pub dimtfillclr: &'a str,
    pub dimtxtdirection: bool,
    pub dimatfit: &'a str,
    pub dimtix: bool,
    pub dimsoxd: bool,
    pub dimtmove: &'a str,
    pub dimupt: bool,
    pub dimtofl: bool,
    pub dimfit: &'a str,
    pub dimdsep: &'a str,
    pub dimrnd: &'a str,
    pub dimzin: &'a str,
    pub dimfrac: &'a str,
    pub dimaunit: &'a str,
    pub dimadec: &'a str,
    pub dimazin: &'a str,
    pub dimalt: bool,
    pub dimaltf: &'a str,
    pub dimaltd: &'a str,
    pub dimaltu: &'a str,
    pub dimalttd: &'a str,
    pub dimaltrnd: &'a str,
    pub dimapost: &'a str,
    pub dimaltz: &'a str,
    pub dimalttz: &'a str,
    pub dimtolj: &'a str,
    pub dimtzin: &'a str,
}

fn btn_s(accent: bool) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |_: &Theme, st| button::Style {
        background: Some(Background::Color(match (accent, st) {
            (true, button::Status::Hovered | button::Status::Pressed) => Color {
                r: 0.20,
                g: 0.42,
                b: 0.72,
                a: 1.0,
            },
            (false, button::Status::Hovered | button::Status::Pressed) => Color {
                r: 0.28,
                g: 0.28,
                b: 0.28,
                a: 1.0,
            },
            (true, _) => ACCENT,
            _ => Color {
                r: 0.22,
                g: 0.22,
                b: 0.22,
                a: 1.0,
            },
        })),
        text_color: TEXT,
        border: Border {
            color: BORDER,
            width: 1.0,
            radius: 4.0.into(),
        },
        ..Default::default()
    }
}

fn tab_btn_style(active: bool) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |_: &Theme, st| button::Style {
        background: Some(Background::Color(match (active, st) {
            (true, _) => ACTIVE,
            (false, button::Status::Hovered | button::Status::Pressed) => Color {
                r: 0.28,
                g: 0.28,
                b: 0.28,
                a: 1.0,
            },
            _ => Color {
                r: 0.20,
                g: 0.20,
                b: 0.20,
                a: 1.0,
            },
        })),
        text_color: TEXT,
        border: Border {
            color: BORDER,
            width: 1.0,
            radius: 3.0.into(),
        },
        ..Default::default()
    }
}

fn list_item_style(active: bool) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |_: &Theme, st| button::Style {
        background: Some(Background::Color(match (active, st) {
            (true, _) => ACTIVE,
            (false, button::Status::Hovered | button::Status::Pressed) => Color {
                r: 0.28,
                g: 0.28,
                b: 0.28,
                a: 1.0,
            },
            _ => Color {
                r: 0.18,
                g: 0.18,
                b: 0.18,
                a: 1.0,
            },
        })),
        text_color: TEXT,
        border: Border {
            color: BORDER,
            width: 0.0,
            radius: 3.0.into(),
        },
        ..Default::default()
    }
}

fn field_style(_: &Theme, _: text_input::Status) -> text_input::Style {
    text_input::Style {
        background: Background::Color(FIELD),
        border: Border {
            color: BORDER,
            width: 1.0,
            radius: 3.0.into(),
        },
        icon: TEXT,
        placeholder: DIM,
        value: TEXT,
        selection: ACCENT,
    }
}

fn hdivider<'a>() -> Element<'a, Message> {
    container(Space::new().width(Fill).height(1))
        .width(Fill)
        .height(1)
        .style(|_: &Theme| container::Style {
            background: Some(Background::Color(BORDER)),
            ..Default::default()
        })
        .into()
}

pub fn view_window<'a>(
    styles: Vec<String>,
    selected: &'a str,
    tab: u8,
    vals: DimStyleValues<'a>,
) -> Element<'a, Message> {
    // ── Toolbar ───────────────────────────────────────────────────────────
    let toolbar = container(
        row![
            button(text("New").size(11))
                .on_press(Message::DimStyleDialogNew)
                .style(btn_s(false))
                .padding([4, 10]),
            button(text("Delete").size(11))
                .on_press(Message::DimStyleDialogDelete)
                .style(btn_s(false))
                .padding([4, 10]),
            button(text("Set Current").size(11))
                .on_press(Message::DimStyleDialogSetCurrent)
                .style(btn_s(false))
                .padding([4, 10]),
            Space::new().width(Fill),
            button(text("Apply").size(11))
                .on_press(Message::DimStyleDialogApply)
                .style(btn_s(true))
                .padding([4, 14]),
            button(text("Close").size(11))
                .on_press(Message::DimStyleDialogClose)
                .style(btn_s(false))
                .padding([4, 10]),
        ]
        .spacing(4)
        .align_y(iced::Center),
    )
    .style(|_: &Theme| container::Style {
        background: Some(Background::Color(TB)),
        ..Default::default()
    })
    .width(Fill)
    .padding([5, 8]);

    // ── Style list panel ──────────────────────────────────────────────────
    let mut list_col = column![].spacing(2);
    for name in &styles {
        let active = name.as_str() == selected;
        list_col = list_col.push(
            button(text(name.clone()).size(11))
                .on_press(Message::DimStyleDialogSelect(name.clone()))
                .style(list_item_style(active))
                .padding([4, 8])
                .width(Fill),
        );
    }
    let style_list = container(
        column![
            text("Styles").size(10).color(DIM),
            container(scrollable(list_col).height(Fill))
                .style(|_: &Theme| container::Style {
                    background: Some(Background::Color(LIST)),
                    border: Border {
                        color: BORDER,
                        width: 1.0,
                        radius: 3.0.into()
                    },
                    ..Default::default()
                })
                .width(180)
                .height(Fill)
                .padding(2),
        ]
        .spacing(4)
        .height(Fill),
    )
    .width(180)
    .height(Fill)
    .padding(iced::Padding {
        top: 12.0,
        right: 8.0,
        bottom: 12.0,
        left: 12.0,
    });

    // ── Tab bar ───────────────────────────────────────────────────────────
    let tabs = row![
        button(text("Lines").size(11))
            .on_press(Message::DimStyleDialogTab(0))
            .style(tab_btn_style(tab == 0))
            .padding([4, 10]),
        button(text("Arrows").size(11))
            .on_press(Message::DimStyleDialogTab(1))
            .style(tab_btn_style(tab == 1))
            .padding([4, 10]),
        button(text("Text").size(11))
            .on_press(Message::DimStyleDialogTab(2))
            .style(tab_btn_style(tab == 2))
            .padding([4, 10]),
        button(text("Scale/Units").size(11))
            .on_press(Message::DimStyleDialogTab(3))
            .style(tab_btn_style(tab == 3))
            .padding([4, 10]),
        button(text("Tolerances").size(11))
            .on_press(Message::DimStyleDialogTab(4))
            .style(tab_btn_style(tab == 4))
            .padding([4, 10]),
        button(text("Alternate").size(11))
            .on_press(Message::DimStyleDialogTab(5))
            .style(tab_btn_style(tab == 5))
            .padding([4, 10]),
    ]
    .spacing(2);

    let lbl = |s: &'static str| text(s).size(11).color(DIM).width(180);

    let mk_field = |fld: DsField, val: &'a str| -> Element<'a, Message> {
        text_input("", val)
            .on_input(move |s| Message::DsEdit(fld.clone(), s))
            .style(field_style)
            .size(11)
            .width(100)
            .into()
    };

    let chk = |label: &'static str, val: bool, fld: DsField| -> Element<'a, Message> {
        checkbox(val)
            .label(label)
            .on_toggle(move |_| Message::DsToggle(fld.clone()))
            .size(14)
            .text_size(11)
            .into()
    };

    let tab_content: Element<'_, Message> = match tab {
        0 => column![
            text("Dimension Line").size(11).color(ACCENT),
            row![
                lbl("Extension (DIMDLE)"),
                mk_field(DsField::Dimdle, vals.dimdle)
            ]
            .spacing(8)
            .align_y(iced::Center),
            row![
                lbl("Spacing (DIMDLI)"),
                mk_field(DsField::Dimdli, vals.dimdli)
            ]
            .spacing(8)
            .align_y(iced::Center),
            row![
                lbl("Text gap (DIMGAP)"),
                mk_field(DsField::Dimgap, vals.dimgap)
            ]
            .spacing(8)
            .align_y(iced::Center),
            chk("Suppress 1st line (DIMSD1)", vals.dimsd1, DsField::Dimsd1),
            chk("Suppress 2nd line (DIMSD2)", vals.dimsd2, DsField::Dimsd2),
            text("Extension Line").size(11).color(ACCENT),
            row![
                lbl("Extension (DIMEXE)"),
                mk_field(DsField::Dimexe, vals.dimexe)
            ]
            .spacing(8)
            .align_y(iced::Center),
            row![
                lbl("Offset (DIMEXO)"),
                mk_field(DsField::Dimexo, vals.dimexo)
            ]
            .spacing(8)
            .align_y(iced::Center),
            chk("Suppress 1st line (DIMSE1)", vals.dimse1, DsField::Dimse1),
            chk("Suppress 2nd line (DIMSE2)", vals.dimse2, DsField::Dimse2),
            row![lbl("Dim line color ACI (DIMCLRD)"), mk_field(DsField::Dimclrd, vals.dimclrd)].spacing(8).align_y(iced::Center),
            row![lbl("Dim line weight (DIMLWD)"), mk_field(DsField::Dimlwd, vals.dimlwd)].spacing(8).align_y(iced::Center),
            row![lbl("Ext line color ACI (DIMCLRE)"), mk_field(DsField::Dimclre, vals.dimclre)].spacing(8).align_y(iced::Center),
            row![lbl("Ext line weight (DIMLWE)"), mk_field(DsField::Dimlwe, vals.dimlwe)].spacing(8).align_y(iced::Center),
            chk("Fixed-length ext lines (DIMFXLON)", vals.dimfxlon, DsField::Dimfxlon),
            row![lbl("Fixed length (DIMFXL)"), mk_field(DsField::Dimfxl, vals.dimfxl)].spacing(8).align_y(iced::Center),
        ]
        .spacing(7)
        .into(),
        1 => column![
            text("Arrows").size(11).color(ACCENT),
            row![
                lbl("Arrow size (DIMASZ)"),
                mk_field(DsField::Dimasz, vals.dimasz)
            ]
            .spacing(8)
            .align_y(iced::Center),
            row![
                lbl("Center mark (DIMCEN)"),
                mk_field(DsField::Dimcen, vals.dimcen)
            ]
            .spacing(8)
            .align_y(iced::Center),
            row![
                lbl("Tick size (DIMTSZ)"),
                mk_field(DsField::Dimtsz, vals.dimtsz)
            ]
            .spacing(8)
            .align_y(iced::Center),
            chk("Separate arrow blocks (DIMSAH)", vals.dimsah, DsField::Dimsah),
            row![lbl("Arc length symbol (DIMARCSYM)"), mk_field(DsField::Dimarcsym, vals.dimarcsym)].spacing(8).align_y(iced::Center),
            row![lbl("Jog angle ° (DIMJOGANG)"), mk_field(DsField::Dimjogang, vals.dimjogang)].spacing(8).align_y(iced::Center),
        ]
        .spacing(7)
        .into(),
        2 => column![
            text("Text").size(11).color(ACCENT),
            row![
                lbl("Height (DIMTXT)"),
                mk_field(DsField::Dimtxt, vals.dimtxt)
            ]
            .spacing(8)
            .align_y(iced::Center),
            row![
                lbl("Style (DIMTXSTY)"),
                mk_field(DsField::Dimtxsty, vals.dimtxsty)
            ]
            .spacing(8)
            .align_y(iced::Center),
            row![
                lbl("Vertical pos (DIMTAD)"),
                mk_field(DsField::Dimtad, vals.dimtad)
            ]
            .spacing(8)
            .align_y(iced::Center),
            chk("Horizontal inside (DIMTIH)", vals.dimtih, DsField::Dimtih),
            chk("Horizontal outside (DIMTOH)", vals.dimtoh, DsField::Dimtoh),
            row![lbl("Text color ACI (DIMCLRT)"), mk_field(DsField::Dimclrt, vals.dimclrt)].spacing(8).align_y(iced::Center),
            row![lbl("Horizontal just (DIMJUST)"), mk_field(DsField::Dimjust, vals.dimjust)].spacing(8).align_y(iced::Center),
            row![lbl("Vertical pos (DIMTVP)"), mk_field(DsField::Dimtvp, vals.dimtvp)].spacing(8).align_y(iced::Center),
            row![lbl("Text fill mode (DIMTFILL)"), mk_field(DsField::Dimtfill, vals.dimtfill)].spacing(8).align_y(iced::Center),
            row![lbl("Fill color ACI (DIMTFILLCLR)"), mk_field(DsField::Dimtfillclr, vals.dimtfillclr)].spacing(8).align_y(iced::Center),
            chk("Left-to-right (DIMTXTDIRECTION)", vals.dimtxtdirection, DsField::Dimtxtdirection),
        ]
        .spacing(7)
        .into(),
        3 => column![
            text("Scale").size(11).color(ACCENT),
            chk("Annotative", vals.annotative, DsField::Annotative),
            row![
                lbl("Overall scale (DIMSCALE)"),
                mk_field(DsField::Dimscale, vals.dimscale)
            ]
            .spacing(8)
            .align_y(iced::Center),
            row![
                lbl("Linear factor (DIMLFAC)"),
                mk_field(DsField::Dimlfac, vals.dimlfac)
            ]
            .spacing(8)
            .align_y(iced::Center),
            text("Units").size(11).color(ACCENT),
            row![
                lbl("Format (DIMLUNIT)"),
                mk_field(DsField::Dimlunit, vals.dimlunit)
            ]
            .spacing(8)
            .align_y(iced::Center),
            row![
                lbl("Decimals (DIMDEC)"),
                mk_field(DsField::Dimdec, vals.dimdec)
            ]
            .spacing(8)
            .align_y(iced::Center),
            row![
                lbl("Suffix (DIMPOST)"),
                mk_field(DsField::Dimpost, vals.dimpost)
            ]
            .spacing(8)
            .align_y(iced::Center),
            row![lbl("Decimal sep ASCII (DIMDSEP)"), mk_field(DsField::Dimdsep, vals.dimdsep)].spacing(8).align_y(iced::Center),
            row![lbl("Round off (DIMRND)"), mk_field(DsField::Dimrnd, vals.dimrnd)].spacing(8).align_y(iced::Center),
            row![lbl("Zero suppress (DIMZIN)"), mk_field(DsField::Dimzin, vals.dimzin)].spacing(8).align_y(iced::Center),
            row![lbl("Fraction format (DIMFRAC)"), mk_field(DsField::Dimfrac, vals.dimfrac)].spacing(8).align_y(iced::Center),
            row![lbl("Angular unit (DIMAUNIT)"), mk_field(DsField::Dimaunit, vals.dimaunit)].spacing(8).align_y(iced::Center),
            row![lbl("Angular decimals (DIMADEC)"), mk_field(DsField::Dimadec, vals.dimadec)].spacing(8).align_y(iced::Center),
            row![lbl("Angular zero supp (DIMAZIN)"), mk_field(DsField::Dimazin, vals.dimazin)].spacing(8).align_y(iced::Center),
            text("Fit").size(11).color(ACCENT),
            row![lbl("Fit (DIMATFIT)"), mk_field(DsField::Dimatfit, vals.dimatfit)].spacing(8).align_y(iced::Center),
            row![lbl("Text movement (DIMTMOVE)"), mk_field(DsField::Dimtmove, vals.dimtmove)].spacing(8).align_y(iced::Center),
            row![lbl("Fit (legacy DIMFIT)"), mk_field(DsField::Dimfit, vals.dimfit)].spacing(8).align_y(iced::Center),
            chk("Force text inside (DIMTIX)", vals.dimtix, DsField::Dimtix),
            chk("Suppress outside arrows (DIMSOXD)", vals.dimsoxd, DsField::Dimsoxd),
            chk("Place text manually (DIMUPT)", vals.dimupt, DsField::Dimupt),
            chk("Dim line between ext (DIMTOFL)", vals.dimtofl, DsField::Dimtofl),
        ]
        .spacing(7)
        .into(),
        5 => column![
            text("Alternate Units").size(11).color(ACCENT),
            chk("Enable alternate units (DIMALT)", vals.dimalt, DsField::Dimalt),
            row![lbl("Multiplier (DIMALTF)"), mk_field(DsField::Dimaltf, vals.dimaltf)].spacing(8).align_y(iced::Center),
            row![lbl("Decimals (DIMALTD)"), mk_field(DsField::Dimaltd, vals.dimaltd)].spacing(8).align_y(iced::Center),
            row![lbl("Unit format (DIMALTU)"), mk_field(DsField::Dimaltu, vals.dimaltu)].spacing(8).align_y(iced::Center),
            row![lbl("Tol decimals (DIMALTTD)"), mk_field(DsField::Dimalttd, vals.dimalttd)].spacing(8).align_y(iced::Center),
            row![lbl("Round off (DIMALTRND)"), mk_field(DsField::Dimaltrnd, vals.dimaltrnd)].spacing(8).align_y(iced::Center),
            row![lbl("Suffix (DIMAPOST)"), mk_field(DsField::Dimapost, vals.dimapost)].spacing(8).align_y(iced::Center),
            row![lbl("Zero suppress (DIMALTZ)"), mk_field(DsField::Dimaltz, vals.dimaltz)].spacing(8).align_y(iced::Center),
            row![lbl("Tol zero supp (DIMALTTZ)"), mk_field(DsField::Dimalttz, vals.dimalttz)].spacing(8).align_y(iced::Center),
        ]
        .spacing(7)
        .into(),
        _ => column![
            text("Tolerances").size(11).color(ACCENT),
            chk("Generate tolerances (DIMTOL)", vals.dimtol, DsField::Dimtol),
            chk("Limits generation (DIMLIM)", vals.dimlim, DsField::Dimlim),
            row![
                lbl("Plus tolerance (DIMTP)"),
                mk_field(DsField::Dimtp, vals.dimtp)
            ]
            .spacing(8)
            .align_y(iced::Center),
            row![
                lbl("Minus tolerance (DIMTM)"),
                mk_field(DsField::Dimtm, vals.dimtm)
            ]
            .spacing(8)
            .align_y(iced::Center),
            row![
                lbl("Tol. decimals (DIMTDEC)"),
                mk_field(DsField::Dimtdec, vals.dimtdec)
            ]
            .spacing(8)
            .align_y(iced::Center),
            row![
                lbl("Tol. scale (DIMTFAC)"),
                mk_field(DsField::Dimtfac, vals.dimtfac)
            ]
            .spacing(8)
            .align_y(iced::Center),
            row![lbl("Tol. vert just (DIMTOLJ)"), mk_field(DsField::Dimtolj, vals.dimtolj)].spacing(8).align_y(iced::Center),
            row![lbl("Tol. zero supp (DIMTZIN)"), mk_field(DsField::Dimtzin, vals.dimtzin)].spacing(8).align_y(iced::Center),
        ]
        .spacing(7)
        .into(),
    };

    // ── Right panel: tabs + scrollable content ────────────────────────────
    let right_panel = container(
        column![
            text(format!("Editing: {selected}")).size(11).color(DIM),
            tabs,
            hdivider(),
            scrollable(container(tab_content).padding([12, 12])).height(Fill),
        ]
        .spacing(6)
        .height(Fill),
    )
    .height(Fill)
    .width(Fill)
    .padding(iced::Padding {
        top: 12.0,
        right: 12.0,
        bottom: 12.0,
        left: 0.0,
    });

    // ── Vertical separator ────────────────────────────────────────────────
    let vsep = container(Space::new().width(1).height(Fill))
        .width(1)
        .height(Fill)
        .style(|_: &Theme| container::Style {
            background: Some(Background::Color(BORDER)),
            ..Default::default()
        });

    let body = row![style_list, vsep, right_panel].height(Fill);

    container(column![toolbar, hdivider(), body].spacing(0))
        .style(|_: &Theme| container::Style {
            background: Some(Background::Color(BG)),
            ..Default::default()
        })
        .width(Fill)
        .height(Fill)
        .into()
}
