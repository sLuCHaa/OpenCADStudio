use super::*;

impl OpenCADStudio {
    pub(super) fn dispatch_styleprops(&mut self, cmd: &str, i: usize) -> Option<Task<Message>> {
        match cmd {
            // COLOR <ByLayer|ByBlock|1-255|name> — the colour applied to new
            // objects (CECOLOR). Bare COLOR reports the current value.
            cmd if cmd == "COLOR"
                || cmd == "COLOUR"
                || cmd == "CECOLOR"
                || cmd == "DDCOLOR"
                || cmd == "BYLAYER"
                || cmd.starts_with("COLOR ")
                || cmd.starts_with("COLOUR ")
                || cmd.starts_with("CECOLOR ")
                || cmd.starts_with("DDCOLOR ") =>
            {
                use acadrust::types::Color;
                let describe = |c: &Color| match c {
                    Color::ByLayer => "ByLayer".to_string(),
                    Color::ByBlock => "ByBlock".to_string(),
                    Color::Index(n) => format!("index {n}"),
                    _ => "(custom)".to_string(),
                };
                let arg = if cmd == "BYLAYER" {
                    "BYLAYER".to_string()
                } else {
                    cmd.split_whitespace().nth(1).unwrap_or("").to_uppercase()
                };
                let color = match arg.as_str() {
                    "" => {
                        let c = self.tabs[i].scene.document.header.current_entity_color;
                        self.command_line
                            .push_output(&format!("Current object colour: {}", describe(&c)));
                        return Some(Task::none());
                    }
                    "BYLAYER" => Some(Color::ByLayer),
                    "BYBLOCK" => Some(Color::ByBlock),
                    "RED" => Some(Color::Index(1)),
                    "YELLOW" => Some(Color::Index(2)),
                    "GREEN" => Some(Color::Index(3)),
                    "CYAN" => Some(Color::Index(4)),
                    "BLUE" => Some(Color::Index(5)),
                    "MAGENTA" => Some(Color::Index(6)),
                    "WHITE" => Some(Color::Index(7)),
                    n => n.parse::<i16>().ok().map(Color::from_index),
                };
                match color {
                    Some(c) => {
                        self.tabs[i].scene.document.header.current_entity_color = c;
                        self.ribbon.active_color = c;
                        self.tabs[i].dirty = true;
                        self.command_line
                            .push_output(&format!("Object colour set to {}.", describe(&c)));
                    }
                    None => {
                        self.command_line.push_error(
                            "Usage: COLOR <ByLayer|ByBlock|1-255|red|yellow|green|cyan|blue|magenta|white>",
                        );
                    }
                }
            }

            // ── LINETYPE management ───────────────────────────────────────
            cmd if cmd == "LINETYPE"
                || cmd == "LT"
                || cmd.starts_with("LINETYPE ")
                || cmd.starts_with("LT ") =>
            {
                let raw_rest = cmd.split_once(' ').map(|(_, r)| r.trim()).unwrap_or("");
                let parts: Vec<&str> = raw_rest.split_whitespace().collect();
                let sub = parts.get(0).map(|s| s.to_uppercase()).unwrap_or_default();
                match sub.as_str() {
                    "" | "LIST" | "?" => {
                        let ltypes: Vec<String> = self.tabs[i]
                            .scene
                            .document
                            .line_types
                            .iter()
                            .map(|lt| format!("{} ({})", lt.name, lt.description))
                            .collect();
                        if ltypes.is_empty() {
                            self.command_line.push_output("No linetypes defined.");
                        } else {
                            self.command_line
                                .push_output(&format!("Linetypes: {}", ltypes.join(", ")));
                        }
                    }
                    // Set the current linetype applied to newly drawn entities.
                    "SET" | "CURRENT" | "S" => {
                        let name = parts.get(1).copied().unwrap_or("");
                        if name.is_empty() {
                            self.command_line
                                .push_info("Usage: LINETYPE SET <name | ByLayer | ByBlock>");
                        } else {
                            let canon = if name.eq_ignore_ascii_case("BYLAYER") {
                                Some(("ByLayer".to_string(), acadrust::types::Handle::NULL))
                            } else if name.eq_ignore_ascii_case("BYBLOCK") {
                                Some(("ByBlock".to_string(), acadrust::types::Handle::NULL))
                            } else {
                                self.tabs[i]
                                    .scene
                                    .document
                                    .line_types
                                    .iter()
                                    .find(|lt| lt.name.eq_ignore_ascii_case(name))
                                    .map(|lt| (lt.name.clone(), lt.handle))
                            };
                            match canon {
                                Some((nm, handle)) => {
                                    let h = &mut self.tabs[i].scene.document.header;
                                    h.current_linetype_name = nm.clone();
                                    h.current_linetype_handle = handle;
                                    self.tabs[i].dirty = true;
                                    self.command_line
                                        .push_output(&format!("Current linetype set to {nm}."));
                                }
                                None => {
                                    self.command_line.push_error(&format!(
                                        "LINETYPE: \"{name}\" is not loaded. Use LINETYPE LIST to see available linetypes."
                                    ));
                                }
                            }
                        }
                    }
                    _ => {
                        self.command_line
                            .push_info("Usage: LINETYPE LIST | SET <name>");
                    }
                }
            }

            // ── PURGE unused definitions ──────────────────────────────────
            cmd if cmd == "PURGE" || cmd.starts_with("PURGE ") => {
                let sub = cmd
                    .split_whitespace()
                    .nth(1)
                    .unwrap_or("ALL")
                    .to_uppercase();
                let all = sub == "ALL" || sub.is_empty();

                // Collect names in use (immutable borrows — done in their own scope)
                let used_layers: rustc_hash::FxHashSet<String> = self.tabs[i]
                    .scene
                    .document
                    .entities()
                    .filter_map(|e| {
                        let name = &e.common().layer;
                        if name.is_empty() {
                            None
                        } else {
                            Some(name.clone())
                        }
                    })
                    .collect();
                let used_text_styles: rustc_hash::FxHashSet<String> = self.tabs[i]
                    .scene
                    .document
                    .entities()
                    .filter_map(|e| match e {
                        acadrust::EntityType::Text(t) => Some(t.style.clone()),
                        acadrust::EntityType::MText(t) => Some(t.style.clone()),
                        _ => None,
                    })
                    .filter(|s| !s.is_empty())
                    .collect();
                let used_linetypes: rustc_hash::FxHashSet<String> = self.tabs[i]
                    .scene
                    .document
                    .entities()
                    .filter_map(|e| {
                        let lt = &e.common().linetype;
                        if lt.is_empty() || lt == "ByLayer" || lt == "ByBlock" {
                            None
                        } else {
                            Some(lt.clone())
                        }
                    })
                    .collect();

                // Build removal lists (still immutable)
                let layer_remove: Vec<String> = if all || sub == "LAYERS" {
                    self.tabs[i]
                        .scene
                        .document
                        .layers
                        .iter()
                        .filter(|l| l.name != "0" && !used_layers.contains(&l.name))
                        .map(|l| l.name.clone())
                        .collect()
                } else {
                    vec![]
                };
                let style_remove: Vec<String> = if all || sub == "TEXTSTYLES" || sub == "STYLES" {
                    self.tabs[i]
                        .scene
                        .document
                        .text_styles
                        .iter()
                        .filter(|s| s.name != "Standard" && !used_text_styles.contains(&s.name))
                        .map(|s| s.name.clone())
                        .collect()
                } else {
                    vec![]
                };
                let lt_remove: Vec<String> = if all || sub == "LINETYPES" || sub == "LT" {
                    let standard = ["Continuous", "ByLayer", "ByBlock"];
                    self.tabs[i]
                        .scene
                        .document
                        .line_types
                        .iter()
                        .filter(|lt| {
                            !standard.iter().any(|s| s.eq_ignore_ascii_case(&lt.name))
                                && !used_linetypes.contains(&lt.name)
                        })
                        .map(|lt| lt.name.clone())
                        .collect()
                } else {
                    vec![]
                };

                // Apply removals (mutable)
                let purged = layer_remove.len() + style_remove.len() + lt_remove.len();
                for name in &layer_remove {
                    self.tabs[i].scene.document.layers.remove(name);
                }
                for name in &style_remove {
                    self.tabs[i].scene.document.text_styles.remove(name);
                }
                for name in &lt_remove {
                    self.tabs[i].scene.document.line_types.remove(name);
                }

                if purged > 0 {
                    self.push_undo_snapshot(i, "PURGE");
                    self.tabs[i].dirty = true;
                    self.command_line
                        .push_output(&format!("PURGE: {} definition(s) removed.", purged));
                } else {
                    self.command_line.push_output("PURGE: nothing to purge.");
                }
            }

            // ── CHPROP — change entity properties from command line ───────
            cmd if cmd == "CHPROP" || cmd.starts_with("CHPROP ") => {
                // Usage: CHPROP <property> <value>
                // Applies to currently selected entities.
                // Properties: LAYER, COLOR, LINETYPE, LTSCALE
                let parts: Vec<&str> = cmd.split_whitespace().collect();
                let prop = parts.get(1).map(|s| s.to_uppercase()).unwrap_or_default();
                let value = parts.get(2).map(|s| s.trim()).unwrap_or("").to_string();

                if prop.is_empty() {
                    self.command_line.push_info(
                        "Usage: CHPROP <prop> <val>  (props: LAYER COLOR LINETYPE LTSCALE)",
                    );
                } else {
                    let handles: Vec<_> = self.tabs[i]
                        .scene
                        .selected_entities()
                        .into_iter()
                        .map(|(h, _)| h)
                        .collect();
                    if handles.is_empty() {
                        self.command_line
                            .push_error("CHPROP: no entities selected.");
                    } else {
                        // Validate value early to give clear errors
                        let color_val: Option<acadrust::types::Color> = if prop == "COLOR" {
                            value
                                .parse::<i16>()
                                .ok()
                                .map(acadrust::types::Color::from_index)
                        } else {
                            None
                        };
                        let ltscale_val: Option<f64> = if prop == "LTSCALE" {
                            value.parse().ok()
                        } else {
                            None
                        };
                        let transparency_val: Option<acadrust::types::Transparency> =
                            if prop == "TRANSPARENCY" {
                                value
                                    .parse::<f64>()
                                    .ok()
                                    .map(acadrust::types::Transparency::from_percent)
                            } else {
                                None
                            };

                        if (prop == "COLOR" && color_val.is_none())
                            || (prop == "LTSCALE" && ltscale_val.is_none())
                            || (prop == "TRANSPARENCY" && transparency_val.is_none())
                        {
                            self.command_line.push_error(&format!(
                                "CHPROP: invalid value '{}' for {}.",
                                value, prop
                            ));
                        } else {
                            let mut changed = 0usize;
                            for handle in &handles {
                                if let Some(entity) =
                                    self.tabs[i].scene.document.get_entity_mut(*handle)
                                {
                                    let common = entity.common_mut();
                                    match prop.as_str() {
                                        "LAYER" => {
                                            common.layer = value.clone();
                                            changed += 1;
                                        }
                                        "LINETYPE" | "LT" => {
                                            common.linetype = value.clone();
                                            changed += 1;
                                        }
                                        "LTSCALE" => {
                                            common.linetype_scale = ltscale_val.unwrap();
                                            changed += 1;
                                        }
                                        "COLOR" => {
                                            common.color = color_val.unwrap();
                                            changed += 1;
                                        }
                                        "TRANSPARENCY" => {
                                            common.transparency = transparency_val.unwrap();
                                            changed += 1;
                                        }
                                        _ => {
                                            self.command_line.push_error(&format!(
                                                "CHPROP: unknown property '{}'. Use: LAYER COLOR LINETYPE LTSCALE TRANSPARENCY", prop
                                            ));
                                            break;
                                        }
                                    }
                                }
                            }
                            if changed > 0 {
                                self.push_undo_snapshot(i, "CHPROP");
                                self.tabs[i].dirty = true;
                                self.command_line.push_output(&format!(
                                    "CHPROP: {} entity/entities updated.",
                                    changed
                                ));
                            }
                        }
                    }
                }
            }

            // ── SETBYLAYER — clear color/linetype/lineweight overrides ────
            // Resets the selected entities' direct property overrides back to
            // ByLayer so they follow their layer again.
            "SETBYLAYER" => {
                let handles: Vec<_> = self.tabs[i]
                    .scene
                    .selected_entities()
                    .into_iter()
                    .map(|(h, _)| h)
                    .collect();
                if handles.is_empty() {
                    self.command_line
                        .push_error("SETBYLAYER: select entities first.");
                } else {
                    self.push_undo_snapshot(i, "SETBYLAYER");
                    let mut changed = 0usize;
                    for handle in &handles {
                        if let Some(entity) = self.tabs[i].scene.document.get_entity_mut(*handle) {
                            let common = entity.common_mut();
                            common.color = acadrust::types::Color::ByLayer;
                            common.linetype = "ByLayer".to_string();
                            common.line_weight = acadrust::types::LineWeight::ByLayer;
                            changed += 1;
                        }
                    }
                    self.tabs[i].dirty = true;
                    self.tabs[i].scene.bump_geometry();
                    self.command_line.push_output(&format!(
                        "SETBYLAYER: reset {changed} entity/entities to ByLayer."
                    ));
                }
            }

            // ── OVERKILL — delete duplicate (identical) objects ──────────
            // Removes objects that are identical in geometry AND properties to
            // another object (compared with the handle ignored). Operates on
            // the current selection, or the whole drawing when nothing is
            // selected. Conservative: only exact duplicates are removed.
            "OVERKILL" => {
                use acadrust::Handle;
                let selected: std::collections::HashSet<u64> = self.tabs[i]
                    .scene
                    .selected_entities()
                    .into_iter()
                    .map(|(h, _)| h.value())
                    .collect();
                // Capture (handle, type-name, handle-normalized clone) for each
                // candidate while the document is borrowed immutably.
                let candidates: Vec<(Handle, String, acadrust::EntityType)> = self.tabs[i]
                    .scene
                    .document
                    .entities()
                    .filter(|e| {
                        selected.is_empty() || selected.contains(&e.common().handle.value())
                    })
                    .map(|e| {
                        let key = crate::entities::names::dxf_name(e).to_string();
                        let mut norm = e.clone();
                        norm.common_mut().handle = Handle::NULL;
                        (e.common().handle, key, norm)
                    })
                    .collect();
                // Bucket by (type, layer) so only like objects are compared.
                let mut kept: Vec<(String, acadrust::EntityType)> = Vec::new();
                let mut dups: Vec<Handle> = Vec::new();
                for (h, key, norm) in &candidates {
                    let bucket = format!("{key}\u{0}{}", norm.common().layer);
                    if kept.iter().any(|(b, e)| b == &bucket && e == norm) {
                        dups.push(*h);
                    } else {
                        kept.push((bucket, norm.clone()));
                    }
                }
                if dups.is_empty() {
                    self.command_line
                        .push_output("OVERKILL: no duplicate objects found.");
                } else {
                    let n = dups.len();
                    self.push_undo_snapshot(i, "OVERKILL");
                    self.tabs[i].scene.erase_entities(&dups);
                    self.tabs[i].dirty = true;
                    self.refresh_properties();
                    self.command_line
                        .push_output(&format!("OVERKILL: deleted {n} duplicate object(s)."));
                }
            }

            // ── SETVAR — read / write system variables ───────────────────
            // SETVAR <name>          → report the value
            // SETVAR <name> <value>  → set it
            // SETVAR ?               → list supported variables
            // Numeric / boolean variables are settable; current-layer/linetype/
            // style names are read-only here (use their own commands to change
            // them, which validate the name).
            // System variables typeable directly (e.g. `MIRRTEXT 1`) as well as
            // through SETVAR. (LTSCALE/PDMODE/PDSIZE have their own commands.)
            cmd if matches!(
                cmd.split_whitespace().next().unwrap_or(""),
                "MIRRTEXT"
                    | "ATTREQ"
                    | "ATTDIA"
                    | "DIMASSOC"
                    | "ANGBASE"
                    | "ANGDIR"
                    | "REGENMODE"
                    | "BLIPMODE"
                    | "SPLFRAME"
                    | "DELOBJ"
                    | "PLINEGEN"
                    | "PSLTSCALE"
                    | "DISPSILH"
                    | "WORLDVIEW"
                    | "LIMCHECK"
                    | "DRAGMODE"
                    | "LUNITS"
                    | "LUPREC"
                    | "AUNITS"
                    | "AUPREC"
                    | "THICKNESS"
                    | "ELEVATION"
                    | "INSUNITS"
                    | "SPLINETYPE"
                    | "ISOLINES"
                    | "DIMASO"
                    | "DIMSHO"
                    | "QTEXTMODE"
                    | "PLIMCHECK"
                    | "VISRETAIN"
                    | "USRTIMER"
                    | "ATTMODE"
                    | "COORDS"
                    | "OSMODE"
                    | "PICKSTYLE"
                    | "SPLINESEGS"
                    | "SURFU"
                    | "SURFV"
                    | "SURFTYPE"
                    | "SHADEDGE"
                    | "MAXACTVP"
                    | "CMLJUST"
                    | "TEXTQLTY"
                    | "SORTENTS"
                    | "XCLIPFRAME"
                    | "HALOGAP"
                    | "TRACEWID"
                    | "SKETCHINC"
            ) =>
            {
                return self.dispatch_styleprops(&format!("SETVAR {cmd}"), i);
            }

            cmd if cmd == "SETVAR" || cmd.starts_with("SETVAR ") => {
                let rest = cmd.strip_prefix("SETVAR").unwrap_or("").trim();
                let mut it = rest.splitn(2, char::is_whitespace);
                let name = it.next().unwrap_or("").to_uppercase();
                let value = it.next().map(|s| s.trim().to_string());
                if name.is_empty() || name == "?" {
                    self.command_line.push_info(
                        "SETVAR: LTSCALE CELTSCALE PDMODE PDSIZE TEXTSIZE ORTHOMODE FILLMODE MIRRTEXT ATTREQ ATTDIA DIMASSOC ANGBASE ANGDIR | CLAYER CELTYPE TEXTSTYLE (read-only)",
                    );
                } else {
                    // Parse a boolean given as 0/1 or ON/OFF.
                    let parse_bool = |s: &str| match s.to_uppercase().as_str() {
                        "1" | "ON" | "TRUE" => Some(true),
                        "0" | "OFF" | "FALSE" => Some(false),
                        _ => None,
                    };
                    let outcome: Result<(String, bool), String> = {
                        let h = &mut self.tabs[i].scene.document.header;
                        match name.as_str() {
                            "LTSCALE" => match &value {
                                Some(v) => v
                                    .parse::<f64>()
                                    .map(|x| {
                                        h.linetype_scale = x;
                                        (format!("LTSCALE = {x}"), true)
                                    })
                                    .map_err(|_| "SETVAR: numeric value required.".into()),
                                None => Ok((format!("LTSCALE = {}", h.linetype_scale), false)),
                            },
                            "CELTSCALE" => match &value {
                                Some(v) => v
                                    .parse::<f64>()
                                    .map(|x| {
                                        h.current_entity_linetype_scale = x;
                                        (format!("CELTSCALE = {x}"), true)
                                    })
                                    .map_err(|_| "SETVAR: numeric value required.".into()),
                                None => Ok((
                                    format!("CELTSCALE = {}", h.current_entity_linetype_scale),
                                    false,
                                )),
                            },
                            "PDSIZE" => match &value {
                                Some(v) => v
                                    .parse::<f64>()
                                    .map(|x| {
                                        h.point_display_size = x;
                                        (format!("PDSIZE = {x}"), true)
                                    })
                                    .map_err(|_| "SETVAR: numeric value required.".into()),
                                None => Ok((format!("PDSIZE = {}", h.point_display_size), false)),
                            },
                            "TEXTSIZE" => match &value {
                                Some(v) => v
                                    .parse::<f64>()
                                    .map(|x| {
                                        h.text_height = x;
                                        (format!("TEXTSIZE = {x}"), true)
                                    })
                                    .map_err(|_| "SETVAR: numeric value required.".into()),
                                None => Ok((format!("TEXTSIZE = {}", h.text_height), false)),
                            },
                            "PDMODE" => match &value {
                                Some(v) => v
                                    .parse::<i16>()
                                    .map(|x| {
                                        h.point_display_mode = x;
                                        (format!("PDMODE = {x}"), true)
                                    })
                                    .map_err(|_| "SETVAR: integer value required.".into()),
                                None => Ok((format!("PDMODE = {}", h.point_display_mode), false)),
                            },
                            "ORTHOMODE" => match &value {
                                Some(v) => parse_bool(v)
                                    .map(|b| {
                                        h.ortho_mode = b;
                                        (format!("ORTHOMODE = {}", b as i32), true)
                                    })
                                    .ok_or_else(|| "SETVAR: 0 or 1 required.".into()),
                                None => Ok((format!("ORTHOMODE = {}", h.ortho_mode as i32), false)),
                            },
                            "FILLMODE" => match &value {
                                Some(v) => parse_bool(v)
                                    .map(|b| {
                                        h.fill_mode = b;
                                        (format!("FILLMODE = {}", b as i32), true)
                                    })
                                    .ok_or_else(|| "SETVAR: 0 or 1 required.".into()),
                                None => Ok((format!("FILLMODE = {}", h.fill_mode as i32), false)),
                            },
                            "MIRRTEXT" => match &value {
                                Some(v) => parse_bool(v)
                                    .map(|b| {
                                        h.mirror_text = b;
                                        (format!("MIRRTEXT = {}", b as i32), true)
                                    })
                                    .ok_or_else(|| "SETVAR: 0 or 1 required.".into()),
                                None => Ok((format!("MIRRTEXT = {}", h.mirror_text as i32), false)),
                            },
                            "ATTREQ" => match &value {
                                Some(v) => parse_bool(v)
                                    .map(|b| {
                                        h.attribute_request = b;
                                        (format!("ATTREQ = {}", b as i32), true)
                                    })
                                    .ok_or_else(|| "SETVAR: 0 or 1 required.".into()),
                                None => {
                                    Ok((format!("ATTREQ = {}", h.attribute_request as i32), false))
                                }
                            },
                            "ATTDIA" => match &value {
                                Some(v) => parse_bool(v)
                                    .map(|b| {
                                        h.attribute_dialog = b;
                                        (format!("ATTDIA = {}", b as i32), true)
                                    })
                                    .ok_or_else(|| "SETVAR: 0 or 1 required.".into()),
                                None => {
                                    Ok((format!("ATTDIA = {}", h.attribute_dialog as i32), false))
                                }
                            },
                            "DIMASSOC" => match &value {
                                Some(v) => v
                                    .parse::<i16>()
                                    .map(|x| {
                                        h.dimension_associativity = x;
                                        (format!("DIMASSOC = {x}"), true)
                                    })
                                    .map_err(|_| "SETVAR: integer value required.".into()),
                                None => {
                                    Ok((format!("DIMASSOC = {}", h.dimension_associativity), false))
                                }
                            },
                            "ANGBASE" => match &value {
                                Some(v) => v
                                    .parse::<f64>()
                                    .map(|x| {
                                        h.angle_base = x;
                                        (format!("ANGBASE = {x}"), true)
                                    })
                                    .map_err(|_| "SETVAR: numeric value required.".into()),
                                None => Ok((format!("ANGBASE = {}", h.angle_base), false)),
                            },
                            "ANGDIR" => match &value {
                                Some(v) => v
                                    .parse::<i16>()
                                    .map(|x| {
                                        h.angle_direction = x;
                                        (format!("ANGDIR = {x}"), true)
                                    })
                                    .map_err(|_| "SETVAR: integer value required.".into()),
                                None => Ok((format!("ANGDIR = {}", h.angle_direction), false)),
                            },
                            "REGENMODE" => match &value {
                                Some(v) => parse_bool(v)
                                    .map(|b| {
                                        h.regen_mode = b;
                                        (format!("REGENMODE = {}", b as i32), true)
                                    })
                                    .ok_or_else(|| "SETVAR: 0 or 1 required.".into()),
                                None => Ok((format!("REGENMODE = {}", h.regen_mode as i32), false)),
                            },
                            "BLIPMODE" => match &value {
                                Some(v) => parse_bool(v)
                                    .map(|b| {
                                        h.blip_mode = b;
                                        (format!("BLIPMODE = {}", b as i32), true)
                                    })
                                    .ok_or_else(|| "SETVAR: 0 or 1 required.".into()),
                                None => Ok((format!("BLIPMODE = {}", h.blip_mode as i32), false)),
                            },
                            "SPLFRAME" => match &value {
                                Some(v) => parse_bool(v)
                                    .map(|b| {
                                        h.spline_frame = b;
                                        (format!("SPLFRAME = {}", b as i32), true)
                                    })
                                    .ok_or_else(|| "SETVAR: 0 or 1 required.".into()),
                                None => {
                                    Ok((format!("SPLFRAME = {}", h.spline_frame as i32), false))
                                }
                            },
                            "DELOBJ" => match &value {
                                Some(v) => parse_bool(v)
                                    .map(|b| {
                                        h.delete_objects = b;
                                        (format!("DELOBJ = {}", b as i32), true)
                                    })
                                    .ok_or_else(|| "SETVAR: 0 or 1 required.".into()),
                                None => {
                                    Ok((format!("DELOBJ = {}", h.delete_objects as i32), false))
                                }
                            },
                            "PLINEGEN" => match &value {
                                Some(v) => parse_bool(v)
                                    .map(|b| {
                                        h.polyline_linetype_generation = b;
                                        (format!("PLINEGEN = {}", b as i32), true)
                                    })
                                    .ok_or_else(|| "SETVAR: 0 or 1 required.".into()),
                                None => Ok((
                                    format!("PLINEGEN = {}", h.polyline_linetype_generation as i32),
                                    false,
                                )),
                            },
                            "PSLTSCALE" => match &value {
                                Some(v) => parse_bool(v)
                                    .map(|b| {
                                        h.paper_space_linetype_scaling = b;
                                        (format!("PSLTSCALE = {}", b as i32), true)
                                    })
                                    .ok_or_else(|| "SETVAR: 0 or 1 required.".into()),
                                None => Ok((
                                    format!(
                                        "PSLTSCALE = {}",
                                        h.paper_space_linetype_scaling as i32
                                    ),
                                    false,
                                )),
                            },
                            "DISPSILH" => match &value {
                                Some(v) => parse_bool(v)
                                    .map(|b| {
                                        h.display_silhouette = b;
                                        (format!("DISPSILH = {}", b as i32), true)
                                    })
                                    .ok_or_else(|| "SETVAR: 0 or 1 required.".into()),
                                None => Ok((
                                    format!("DISPSILH = {}", h.display_silhouette as i32),
                                    false,
                                )),
                            },
                            "WORLDVIEW" => match &value {
                                Some(v) => parse_bool(v)
                                    .map(|b| {
                                        h.world_view = b;
                                        (format!("WORLDVIEW = {}", b as i32), true)
                                    })
                                    .ok_or_else(|| "SETVAR: 0 or 1 required.".into()),
                                None => Ok((format!("WORLDVIEW = {}", h.world_view as i32), false)),
                            },
                            "LIMCHECK" => match &value {
                                Some(v) => parse_bool(v)
                                    .map(|b| {
                                        h.limit_check = b;
                                        (format!("LIMCHECK = {}", b as i32), true)
                                    })
                                    .ok_or_else(|| "SETVAR: 0 or 1 required.".into()),
                                None => Ok((format!("LIMCHECK = {}", h.limit_check as i32), false)),
                            },
                            "DRAGMODE" => match &value {
                                Some(v) => v
                                    .parse::<i16>()
                                    .map(|x| {
                                        h.drag_mode = x;
                                        (format!("DRAGMODE = {x}"), true)
                                    })
                                    .map_err(|_| "SETVAR: integer value required.".into()),
                                None => Ok((format!("DRAGMODE = {}", h.drag_mode), false)),
                            },
                            "LUNITS" => match &value {
                                Some(v) => v
                                    .parse::<i16>()
                                    .map(|x| {
                                        h.linear_unit_format = x;
                                        (format!("LUNITS = {x}"), true)
                                    })
                                    .map_err(|_| "SETVAR: integer value required.".into()),
                                None => Ok((format!("LUNITS = {}", h.linear_unit_format), false)),
                            },
                            "LUPREC" => match &value {
                                Some(v) => v
                                    .parse::<i16>()
                                    .map(|x| {
                                        h.linear_unit_precision = x;
                                        (format!("LUPREC = {x}"), true)
                                    })
                                    .map_err(|_| "SETVAR: integer value required.".into()),
                                None => {
                                    Ok((format!("LUPREC = {}", h.linear_unit_precision), false))
                                }
                            },
                            "AUNITS" => match &value {
                                Some(v) => v
                                    .parse::<i16>()
                                    .map(|x| {
                                        h.angular_unit_format = x;
                                        (format!("AUNITS = {x}"), true)
                                    })
                                    .map_err(|_| "SETVAR: integer value required.".into()),
                                None => Ok((format!("AUNITS = {}", h.angular_unit_format), false)),
                            },
                            "AUPREC" => match &value {
                                Some(v) => v
                                    .parse::<i16>()
                                    .map(|x| {
                                        h.angular_unit_precision = x;
                                        (format!("AUPREC = {x}"), true)
                                    })
                                    .map_err(|_| "SETVAR: integer value required.".into()),
                                None => {
                                    Ok((format!("AUPREC = {}", h.angular_unit_precision), false))
                                }
                            },
                            "THICKNESS" => match &value {
                                Some(v) => v
                                    .parse::<f64>()
                                    .map(|x| {
                                        h.thickness = x;
                                        (format!("THICKNESS = {x}"), true)
                                    })
                                    .map_err(|_| "SETVAR: numeric value required.".into()),
                                None => Ok((format!("THICKNESS = {}", h.thickness), false)),
                            },
                            "ELEVATION" => match &value {
                                Some(v) => v
                                    .parse::<f64>()
                                    .map(|x| {
                                        h.elevation = x;
                                        (format!("ELEVATION = {x}"), true)
                                    })
                                    .map_err(|_| "SETVAR: numeric value required.".into()),
                                None => Ok((format!("ELEVATION = {}", h.elevation), false)),
                            },
                            "INSUNITS" => match &value {
                                Some(v) => v
                                    .parse::<i16>()
                                    .map(|x| {
                                        h.insertion_units = x;
                                        (format!("INSUNITS = {x}"), true)
                                    })
                                    .map_err(|_| "SETVAR: integer value required.".into()),
                                None => Ok((format!("INSUNITS = {}", h.insertion_units), false)),
                            },
                            "SPLINETYPE" => match &value {
                                Some(v) => v
                                    .parse::<i16>()
                                    .map(|x| {
                                        h.spline_type = x;
                                        (format!("SPLINETYPE = {x}"), true)
                                    })
                                    .map_err(|_| "SETVAR: integer value required.".into()),
                                None => Ok((format!("SPLINETYPE = {}", h.spline_type), false)),
                            },
                            "ISOLINES" => match &value {
                                Some(v) => v
                                    .parse::<i16>()
                                    .map(|x| {
                                        h.isolines = x;
                                        (format!("ISOLINES = {x}"), true)
                                    })
                                    .map_err(|_| "SETVAR: integer value required.".into()),
                                None => Ok((format!("ISOLINES = {}", h.isolines), false)),
                            },
                            "DIMASO" => match &value {
                                Some(v) => parse_bool(v)
                                    .map(|b| {
                                        h.associate_dimensions = b;
                                        (format!("DIMASO = {}", b as i32), true)
                                    })
                                    .ok_or_else(|| "SETVAR: 0 or 1 required.".into()),
                                None => Ok((
                                    format!("DIMASO = {}", h.associate_dimensions as i32),
                                    false,
                                )),
                            },
                            "DIMSHO" => match &value {
                                Some(v) => parse_bool(v)
                                    .map(|b| {
                                        h.update_dimensions_while_dragging = b;
                                        (format!("DIMSHO = {}", b as i32), true)
                                    })
                                    .ok_or_else(|| "SETVAR: 0 or 1 required.".into()),
                                None => Ok((
                                    format!(
                                        "DIMSHO = {}",
                                        h.update_dimensions_while_dragging as i32
                                    ),
                                    false,
                                )),
                            },
                            "QTEXTMODE" => match &value {
                                Some(v) => parse_bool(v)
                                    .map(|b| {
                                        h.quick_text_mode = b;
                                        (format!("QTEXTMODE = {}", b as i32), true)
                                    })
                                    .ok_or_else(|| "SETVAR: 0 or 1 required.".into()),
                                None => {
                                    Ok((format!("QTEXTMODE = {}", h.quick_text_mode as i32), false))
                                }
                            },
                            "PLIMCHECK" => match &value {
                                Some(v) => parse_bool(v)
                                    .map(|b| {
                                        h.paper_space_limit_check = b;
                                        (format!("PLIMCHECK = {}", b as i32), true)
                                    })
                                    .ok_or_else(|| "SETVAR: 0 or 1 required.".into()),
                                None => Ok((
                                    format!("PLIMCHECK = {}", h.paper_space_limit_check as i32),
                                    false,
                                )),
                            },
                            "VISRETAIN" => match &value {
                                Some(v) => parse_bool(v)
                                    .map(|b| {
                                        h.retain_xref_visibility = b;
                                        (format!("VISRETAIN = {}", b as i32), true)
                                    })
                                    .ok_or_else(|| "SETVAR: 0 or 1 required.".into()),
                                None => Ok((
                                    format!("VISRETAIN = {}", h.retain_xref_visibility as i32),
                                    false,
                                )),
                            },
                            "USRTIMER" => match &value {
                                Some(v) => parse_bool(v)
                                    .map(|b| {
                                        h.user_timer = b;
                                        (format!("USRTIMER = {}", b as i32), true)
                                    })
                                    .ok_or_else(|| "SETVAR: 0 or 1 required.".into()),
                                None => Ok((format!("USRTIMER = {}", h.user_timer as i32), false)),
                            },
                            "ATTMODE" => match &value {
                                Some(v) => v
                                    .parse::<i16>()
                                    .map(|x| {
                                        h.attribute_visibility = x;
                                        (format!("ATTMODE = {x}"), true)
                                    })
                                    .map_err(|_| "SETVAR: integer value required.".into()),
                                None => {
                                    Ok((format!("ATTMODE = {}", h.attribute_visibility), false))
                                }
                            },
                            "COORDS" => match &value {
                                Some(v) => v
                                    .parse::<i16>()
                                    .map(|x| {
                                        h.coords_mode = x;
                                        (format!("COORDS = {x}"), true)
                                    })
                                    .map_err(|_| "SETVAR: integer value required.".into()),
                                None => Ok((format!("COORDS = {}", h.coords_mode), false)),
                            },
                            "OSMODE" => match &value {
                                Some(v) => v
                                    .parse::<i32>()
                                    .map(|x| {
                                        h.object_snap_mode = x;
                                        (format!("OSMODE = {x}"), true)
                                    })
                                    .map_err(|_| "SETVAR: integer value required.".into()),
                                None => Ok((format!("OSMODE = {}", h.object_snap_mode), false)),
                            },
                            "PICKSTYLE" => match &value {
                                Some(v) => v
                                    .parse::<i16>()
                                    .map(|x| {
                                        h.pick_style = x;
                                        (format!("PICKSTYLE = {x}"), true)
                                    })
                                    .map_err(|_| "SETVAR: integer value required.".into()),
                                None => Ok((format!("PICKSTYLE = {}", h.pick_style), false)),
                            },
                            "SPLINESEGS" => match &value {
                                Some(v) => v
                                    .parse::<i16>()
                                    .map(|x| {
                                        h.spline_segments = x;
                                        (format!("SPLINESEGS = {x}"), true)
                                    })
                                    .map_err(|_| "SETVAR: integer value required.".into()),
                                None => Ok((format!("SPLINESEGS = {}", h.spline_segments), false)),
                            },
                            "SURFU" => match &value {
                                Some(v) => v
                                    .parse::<i16>()
                                    .map(|x| {
                                        h.surface_u_density = x;
                                        (format!("SURFU = {x}"), true)
                                    })
                                    .map_err(|_| "SETVAR: integer value required.".into()),
                                None => Ok((format!("SURFU = {}", h.surface_u_density), false)),
                            },
                            "SURFV" => match &value {
                                Some(v) => v
                                    .parse::<i16>()
                                    .map(|x| {
                                        h.surface_v_density = x;
                                        (format!("SURFV = {x}"), true)
                                    })
                                    .map_err(|_| "SETVAR: integer value required.".into()),
                                None => Ok((format!("SURFV = {}", h.surface_v_density), false)),
                            },
                            "SURFTYPE" => match &value {
                                Some(v) => v
                                    .parse::<i16>()
                                    .map(|x| {
                                        h.surface_type = x;
                                        (format!("SURFTYPE = {x}"), true)
                                    })
                                    .map_err(|_| "SETVAR: integer value required.".into()),
                                None => Ok((format!("SURFTYPE = {}", h.surface_type), false)),
                            },
                            "SHADEDGE" => match &value {
                                Some(v) => v
                                    .parse::<i16>()
                                    .map(|x| {
                                        h.shade_edge = x;
                                        (format!("SHADEDGE = {x}"), true)
                                    })
                                    .map_err(|_| "SETVAR: integer value required.".into()),
                                None => Ok((format!("SHADEDGE = {}", h.shade_edge), false)),
                            },
                            "MAXACTVP" => match &value {
                                Some(v) => v
                                    .parse::<i16>()
                                    .map(|x| {
                                        h.max_active_viewports = x;
                                        (format!("MAXACTVP = {x}"), true)
                                    })
                                    .map_err(|_| "SETVAR: integer value required.".into()),
                                None => {
                                    Ok((format!("MAXACTVP = {}", h.max_active_viewports), false))
                                }
                            },
                            "CMLJUST" => match &value {
                                Some(v) => v
                                    .parse::<i16>()
                                    .map(|x| {
                                        h.multiline_justification = x;
                                        (format!("CMLJUST = {x}"), true)
                                    })
                                    .map_err(|_| "SETVAR: integer value required.".into()),
                                None => {
                                    Ok((format!("CMLJUST = {}", h.multiline_justification), false))
                                }
                            },
                            "TEXTQLTY" => match &value {
                                Some(v) => v
                                    .parse::<i16>()
                                    .map(|x| {
                                        h.text_quality = x;
                                        (format!("TEXTQLTY = {x}"), true)
                                    })
                                    .map_err(|_| "SETVAR: integer value required.".into()),
                                None => Ok((format!("TEXTQLTY = {}", h.text_quality), false)),
                            },
                            "SORTENTS" => match &value {
                                Some(v) => v
                                    .parse::<i16>()
                                    .map(|x| {
                                        h.sort_entities = x;
                                        (format!("SORTENTS = {x}"), true)
                                    })
                                    .map_err(|_| "SETVAR: integer value required.".into()),
                                None => Ok((format!("SORTENTS = {}", h.sort_entities), false)),
                            },
                            "XCLIPFRAME" => match &value {
                                Some(v) => v
                                    .parse::<i16>()
                                    .map(|x| {
                                        h.xclip_frame = x;
                                        (format!("XCLIPFRAME = {x}"), true)
                                    })
                                    .map_err(|_| "SETVAR: integer value required.".into()),
                                None => Ok((format!("XCLIPFRAME = {}", h.xclip_frame), false)),
                            },
                            "HALOGAP" => match &value {
                                Some(v) => v
                                    .parse::<i16>()
                                    .map(|x| {
                                        h.halo_gap = x;
                                        (format!("HALOGAP = {x}"), true)
                                    })
                                    .map_err(|_| "SETVAR: integer value required.".into()),
                                None => Ok((format!("HALOGAP = {}", h.halo_gap), false)),
                            },
                            "TRACEWID" => match &value {
                                Some(v) => v
                                    .parse::<f64>()
                                    .map(|x| {
                                        h.trace_width = x;
                                        (format!("TRACEWID = {x}"), true)
                                    })
                                    .map_err(|_| "SETVAR: numeric value required.".into()),
                                None => Ok((format!("TRACEWID = {}", h.trace_width), false)),
                            },
                            "SKETCHINC" => match &value {
                                Some(v) => v
                                    .parse::<f64>()
                                    .map(|x| {
                                        h.sketch_increment = x;
                                        (format!("SKETCHINC = {x}"), true)
                                    })
                                    .map_err(|_| "SETVAR: numeric value required.".into()),
                                None => Ok((format!("SKETCHINC = {}", h.sketch_increment), false)),
                            },
                            "CLAYER" => match &value {
                                Some(_) => Err(
                                    "SETVAR: CLAYER is read-only here — use the CLAYER command."
                                        .into(),
                                ),
                                None => Ok((format!("CLAYER = {}", h.current_layer_name), false)),
                            },
                            "CELTYPE" => match &value {
                                Some(_) => {
                                    Err("SETVAR: CELTYPE is read-only here — use LINETYPE SET."
                                        .into())
                                }
                                None => {
                                    Ok((format!("CELTYPE = {}", h.current_linetype_name), false))
                                }
                            },
                            "TEXTSTYLE" => match &value {
                                Some(_) => Err(
                                    "SETVAR: TEXTSTYLE is read-only here — use the STYLE command."
                                        .into(),
                                ),
                                None => Ok((
                                    format!("TEXTSTYLE = {}", h.current_text_style_name),
                                    false,
                                )),
                            },
                            _ => Err(format!("SETVAR: unknown variable \"{name}\".")),
                        }
                    };
                    match outcome {
                        Ok((msg, changed)) => {
                            if changed {
                                self.tabs[i].dirty = true;
                            }
                            self.command_line.push_output(&msg);
                        }
                        Err(e) => self.command_line.push_error(&e),
                    }
                }
            }

            // ── FINDNONPURGEABLE — list named objects still in use ───────
            // Reports the layers, linetypes, text styles and blocks that are
            // referenced by objects, and therefore cannot be purged — so it is
            // clear why PURGE leaves them behind. Read-only.
            "FINDNONPURGEABLE" => {
                use rustc_hash::FxHashSet;
                let mut layers: FxHashSet<String> = FxHashSet::default();
                let mut linetypes: FxHashSet<String> = FxHashSet::default();
                let mut styles: FxHashSet<String> = FxHashSet::default();
                let mut blocks: FxHashSet<String> = FxHashSet::default();
                {
                    let doc = &self.tabs[i].scene.document;
                    for e in doc.entities() {
                        let l = &e.common().layer;
                        if !l.is_empty() {
                            layers.insert(l.clone());
                        }
                        let lt = &e.common().linetype;
                        if !lt.is_empty() && lt != "ByLayer" && lt != "ByBlock" {
                            linetypes.insert(lt.clone());
                        }
                        match e {
                            acadrust::EntityType::Text(t) if !t.style.is_empty() => {
                                styles.insert(t.style.clone());
                            }
                            acadrust::EntityType::MText(t) if !t.style.is_empty() => {
                                styles.insert(t.style.clone());
                            }
                            acadrust::EntityType::Insert(ins) => {
                                blocks.insert(ins.block_name.clone());
                            }
                            _ => {}
                        }
                    }
                }
                let fmt = |set: FxHashSet<String>| -> String {
                    if set.is_empty() {
                        "(none)".to_string()
                    } else {
                        let mut v: Vec<_> = set.into_iter().collect();
                        v.sort();
                        v.join(", ")
                    }
                };
                self.command_line
                    .push_output("FINDNONPURGEABLE: named objects in use (not purgeable):");
                self.command_line
                    .push_output(&format!("  Layers: {}", fmt(layers)));
                self.command_line
                    .push_output(&format!("  Linetypes: {}", fmt(linetypes)));
                self.command_line
                    .push_output(&format!("  Text styles: {}", fmt(styles)));
                self.command_line
                    .push_output(&format!("  Blocks: {}", fmt(blocks)));
            }

            // ── AUDIT — report drawing-database integrity issues ─────────
            // Read-only scan: flags entities on undefined layers and block
            // references to undefined block definitions. Reports only; it does
            // not auto-repair (so it can never make the drawing worse).
            "AUDIT" => {
                use std::collections::BTreeSet;
                let mut undefined_layers: BTreeSet<String> = BTreeSet::new();
                let mut undefined_blocks: BTreeSet<String> = BTreeSet::new();
                let mut total = 0usize;
                {
                    let doc = &self.tabs[i].scene.document;
                    for e in doc.entities() {
                        total += 1;
                        let layer = &e.common().layer;
                        if !layer.is_empty() && doc.layers.get(layer).is_none() {
                            undefined_layers.insert(layer.clone());
                        }
                        if let acadrust::EntityType::Insert(ins) = e {
                            if doc.block_records.get(&ins.block_name).is_none() {
                                undefined_blocks.insert(ins.block_name.clone());
                            }
                        }
                    }
                }
                self.command_line
                    .push_output(&format!("AUDIT: scanned {total} object(s)."));
                if undefined_layers.is_empty() && undefined_blocks.is_empty() {
                    self.command_line.push_output("AUDIT: no issues found.");
                } else {
                    if !undefined_layers.is_empty() {
                        self.command_line.push_error(&format!(
                            "AUDIT: reference(s) to undefined layer(s): {}",
                            undefined_layers.into_iter().collect::<Vec<_>>().join(", ")
                        ));
                    }
                    if !undefined_blocks.is_empty() {
                        self.command_line.push_error(&format!(
                            "AUDIT: reference(s) to undefined block(s): {}",
                            undefined_blocks.into_iter().collect::<Vec<_>>().join(", ")
                        ));
                    }
                    self.command_line
                        .push_info("AUDIT: report only — no automatic repair performed.");
                }
            }

            // ── RENAME table entries ──────────────────────────────────────
            cmd if cmd == "RENAME" || cmd.starts_with("RENAME ") => {
                // Usage: RENAME <type> <old_name> <new_name>
                // Types: LAYER BLOCK STYLE DIMSTYLE LINETYPE UCS VIEW
                let parts: Vec<&str> = cmd.split_whitespace().collect();
                let type_str = parts.get(1).map(|s| s.to_uppercase()).unwrap_or_default();
                let old_name = parts.get(2).map(|s| s.trim()).unwrap_or("").to_string();
                let new_name = parts.get(3).map(|s| s.trim()).unwrap_or("").to_string();

                if type_str.is_empty() || old_name.is_empty() || new_name.is_empty() {
                    self.command_line.push_info(
                        "Usage: RENAME <type> <old> <new>  (types: LAYER BLOCK STYLE DIMSTYLE LINETYPE UCS VIEW)"
                    );
                } else {
                    let doc = &mut self.tabs[i].scene.document;
                    let ok = match type_str.as_str() {
                        "LAYER" => {
                            if let Some(l) = doc.layers.get_mut(&old_name) {
                                l.name = new_name.clone();
                                // Update entity references
                                for e in doc.entities_mut() {
                                    if e.common().layer == old_name {
                                        e.common_mut().layer = new_name.clone();
                                    }
                                }
                                true
                            } else {
                                false
                            }
                        }
                        "STYLE" | "TEXTSTYLE" => {
                            if let Some(s) = doc.text_styles.get_mut(&old_name) {
                                s.name = new_name.clone();
                                true
                            } else {
                                false
                            }
                        }
                        "DIMSTYLE" => {
                            if let Some(s) = doc.dim_styles.get_mut(&old_name) {
                                s.name = new_name.clone();
                                true
                            } else {
                                false
                            }
                        }
                        "LINETYPE" | "LT" => {
                            if let Some(lt) = doc.line_types.get_mut(&old_name) {
                                lt.name = new_name.clone();
                                true
                            } else {
                                false
                            }
                        }
                        "UCS" => {
                            if let Some(u) = doc.ucss.get_mut(&old_name) {
                                u.name = new_name.clone();
                                true
                            } else {
                                false
                            }
                        }
                        "VIEW" => {
                            if let Some(v) = doc.views.get_mut(&old_name) {
                                v.name = new_name.clone();
                                true
                            } else {
                                false
                            }
                        }
                        _ => {
                            self.command_line.push_error(&format!("RENAME: unknown type '{}'. Use LAYER BLOCK STYLE DIMSTYLE LINETYPE UCS VIEW", type_str));
                            false
                        }
                    };
                    if ok {
                        self.push_undo_snapshot(i, "RENAME");
                        self.tabs[i].dirty = true;
                        self.command_line
                            .push_output(&format!("RENAME: '{}' → '{}'.", old_name, new_name));
                    } else if type_str != "BLOCK" {
                        self.command_line.push_error(&format!(
                            "RENAME: '{}' not found in {}.",
                            old_name, type_str
                        ));
                    }
                }
            }

            // ── System variable getters/setters ──────────────────────────────────
            // CLAYER [name]    — get or set current layer
            // TEXTSTYLE [name] — already handled above under STYLE SET
            // DIMSTYLE [name]  — get or set active dim style
            // LTSCALE [val]    — global linetype scale
            cmd if cmd == "CLAYER" || cmd.starts_with("CLAYER ") => {
                let name_arg = cmd.trim_start_matches("CLAYER").trim();
                if name_arg.is_empty() {
                    let cur = &self.tabs[i].scene.document.header.current_layer_name;
                    self.command_line
                        .push_output(&format!("CLAYER = \"{cur}\""));
                } else {
                    if self.tabs[i].scene.document.layers.contains(name_arg) {
                        self.tabs[i].scene.document.header.current_layer_name =
                            name_arg.to_string();
                        self.tabs[i].dirty = true;
                        self.command_line
                            .push_output(&format!("CLAYER set to \"{name_arg}\""));
                    } else {
                        self.command_line
                            .push_error(&format!("CLAYER: layer '{}' not found.", name_arg));
                    }
                }
            }
            cmd if cmd == "CDIMSTY"
                || cmd == "DIMCURRENT"
                || cmd.starts_with("CDIMSTY ")
                || cmd.starts_with("DIMCURRENT ") =>
            {
                let name_arg = cmd.split_whitespace().skip(1).collect::<Vec<_>>().join(" ");
                if name_arg.is_empty() {
                    let cur = &self.tabs[i].scene.document.header.current_dimstyle_name;
                    self.command_line
                        .push_output(&format!("CDIMSTY = \"{cur}\""));
                } else {
                    if self.tabs[i].scene.document.dim_styles.contains(&name_arg) {
                        self.tabs[i].scene.document.header.current_dimstyle_name = name_arg.clone();
                        self.tabs[i].dirty = true;
                        self.command_line
                            .push_output(&format!("Active dim style set to \"{name_arg}\""));
                    } else {
                        self.command_line
                            .push_error(&format!("CDIMSTY: dim style '{}' not found.", name_arg));
                    }
                }
            }
            "LTSCALE" => {
                use crate::command::ValuePromptCommand;
                let c = ValuePromptCommand::new("LTSCALE", "LTSCALE  new global line-type scale:");
                self.command_line.push_info(&c.prompt());
                self.tabs[i].active_cmd = Some(Box::new(c));
            }
            cmd if cmd.starts_with("LTSCALE ") => {
                let val_str = cmd.trim_start_matches("LTSCALE").trim();
                if val_str.is_empty() {
                    let v = self.tabs[i].scene.document.header.linetype_scale;
                    self.command_line.push_output(&format!("LTSCALE = {v:.4}"));
                } else if let Ok(v) = val_str.parse::<f64>() {
                    if v > 0.0 {
                        self.push_undo_snapshot(i, "LTSCALE");
                        self.tabs[i].scene.document.header.linetype_scale = v;
                        self.tabs[i].dirty = true;
                        self.command_line
                            .push_output(&format!("LTSCALE set to {v:.4}"));
                    } else {
                        self.command_line
                            .push_error("LTSCALE: value must be positive.");
                    }
                } else {
                    self.command_line.push_error("Usage: LTSCALE [value]");
                }
            }
            "PDMODE" => {
                use crate::command::ValuePromptCommand;
                let c = ValuePromptCommand::new(
                    "PDMODE",
                    "PDMODE  new value [0=dot 1=none 2=+ 3=x 4=tick; +32 circle +64 square]:",
                );
                self.command_line.push_info(&c.prompt());
                self.tabs[i].active_cmd = Some(Box::new(c));
            }
            cmd if cmd.starts_with("PDMODE ") => {
                let val_str = cmd.trim_start_matches("PDMODE").trim();
                if val_str.is_empty() {
                    let v = self.tabs[i].scene.document.header.point_display_mode;
                    self.command_line.push_output(&format!("PDMODE = {v}"));
                } else if let Ok(v) = val_str.parse::<i16>() {
                    self.push_undo_snapshot(i, "PDMODE");
                    self.tabs[i].scene.document.header.point_display_mode = v;
                    // Point glyphs are built at tessellation time — rebuild them.
                    self.tabs[i].scene.bump_geometry();
                    self.tabs[i].dirty = true;
                    self.command_line.push_output(&format!("PDMODE set to {v}"));
                } else {
                    self.command_line.push_error(
                        "Usage: PDMODE [value]  (0=dot 1=none 2=+ 3=x 4=tick; +32 circle, +64 square)",
                    );
                }
            }
            cmd if cmd.starts_with("TEXTEDITMODE ") => {
                let val_str = cmd.trim_start_matches("TEXTEDITMODE").trim().to_lowercase();
                if val_str.is_empty() {
                    let v = if self.texteditmode { 1 } else { 0 };
                    self.command_line
                        .push_output(&format!("TEXTEDITMODE = {v}"));
                } else if let Some(v) =
                    crate::modules::annotate::textedit::parse_texteditmode(&val_str)
                {
                    self.texteditmode = v;
                    let n = if v { 1 } else { 0 };
                    self.command_line
                        .push_output(&format!("TEXTEDITMODE set to {n}"));
                } else {
                    self.command_line
                        .push_error("Requires 0 OR 1 OR MULTIPLE OR SINGLE");
                }
            }
            "ISAVEBAK" => {
                let v = if self.backup_on_save { 1 } else { 0 };
                self.command_line.push_output(&format!("ISAVEBAK = {v}"));
            }
            cmd if cmd.starts_with("ISAVEBAK ") => {
                match cmd.trim_start_matches("ISAVEBAK").trim() {
                    "0" => {
                        self.backup_on_save = false;
                        self.persist_settings_if_changed();
                        self.command_line.push_output("ISAVEBAK set to 0");
                    }
                    "1" => {
                        self.backup_on_save = true;
                        self.persist_settings_if_changed();
                        self.command_line.push_output("ISAVEBAK set to 1");
                    }
                    _ => self.command_line.push_error("Requires 0 or 1"),
                }
            }
            "FILEASSOC" => {
                let v = if self.file_assoc_enabled { 1 } else { 0 };
                self.command_line.push_output(&format!("FILEASSOC = {v}"));
            }
            cmd if cmd.starts_with("FILEASSOC ") => {
                match cmd.trim_start_matches("FILEASSOC").trim() {
                    "1" => {
                        self.file_assoc_enabled = true;
                        self.persist_settings_if_changed();
                        match crate::io::file_association::register_as_handler() {
                            Ok(()) => self.command_line.push_output(
                                "FILEASSOC set to 1 — registered as a .dwg/.dxf/.bak handler",
                            ),
                            Err(e) => self
                                .command_line
                                .push_error(&format!("FILEASSOC: registration failed: {e}")),
                        }
                    }
                    "0" => {
                        self.file_assoc_enabled = false;
                        self.persist_settings_if_changed();
                        match crate::io::file_association::unregister_handler() {
                            Ok(()) => self
                                .command_line
                                .push_output("FILEASSOC set to 0 — unregistered as a file handler"),
                            Err(e) => self
                                .command_line
                                .push_error(&format!("FILEASSOC: unregister failed: {e}")),
                        }
                    }
                    _ => self.command_line.push_error("Requires 0 or 1"),
                }
            }
            "PDSIZE" => {
                use crate::command::ValuePromptCommand;
                let c = ValuePromptCommand::new(
                    "PDSIZE",
                    "PDSIZE  new point size (0 = 5% of viewport, <0 = absolute):",
                );
                self.command_line.push_info(&c.prompt());
                self.tabs[i].active_cmd = Some(Box::new(c));
            }
            cmd if cmd.starts_with("PDSIZE ") => {
                let val_str = cmd.trim_start_matches("PDSIZE").trim();
                if val_str.is_empty() {
                    let v = self.tabs[i].scene.document.header.point_display_size;
                    self.command_line.push_output(&format!("PDSIZE = {v:.4}"));
                } else if let Ok(v) = val_str.parse::<f64>() {
                    self.push_undo_snapshot(i, "PDSIZE");
                    self.tabs[i].scene.document.header.point_display_size = v;
                    self.tabs[i].scene.bump_geometry();
                    self.tabs[i].dirty = true;
                    self.command_line
                        .push_output(&format!("PDSIZE set to {v:.4}"));
                } else {
                    self.command_line.push_error(
                        "Usage: PDSIZE [value]  (>0 absolute size, <0 percent of viewport, 0 default)",
                    );
                }
            }
            cmd if cmd == "DDPTYPE" => {
                // The dialog shows the magnitude; the sign (relative/absolute)
                // is driven by the radio buttons. A positive PDSIZE is absolute;
                // zero or negative is relative.
                let pdsize = self.tabs[i].scene.document.header.point_display_size;
                self.point_size_relative = pdsize <= 0.0;
                self.point_size_buf = format!("{}", pdsize.abs());
                self.active_modal = Some(super::super::ModalKind::PointStyle);
            }
            cmd if cmd == "LWDISPLAY" || cmd.starts_with("LWDISPLAY ") => {
                let val_str = cmd.trim_start_matches("LWDISPLAY").trim();
                let parsed: Result<Option<bool>, ()> = match val_str.to_ascii_uppercase().as_str() {
                    "" => Ok(None),
                    "ON" | "1" | "TRUE" => Ok(Some(true)),
                    "OFF" | "0" | "FALSE" => Ok(Some(false)),
                    _ => Err(()),
                };
                match parsed {
                    Err(_) => self.command_line.push_error("Usage: LWDISPLAY [ON|OFF]"),
                    Ok(Some(v)) => {
                        self.push_undo_snapshot(i, "LWDISPLAY");
                        self.tabs[i].scene.document.header.lineweight_display = v;
                        // No retessellate — the wire shader honours the flag via uniforms.
                        self.tabs[i].dirty = true;
                        self.command_line
                            .push_output(&format!("LWDISPLAY {}", if v { "ON" } else { "OFF" }));
                    }
                    Ok(None) => {
                        let v = self.tabs[i].scene.document.header.lineweight_display;
                        self.command_line
                            .push_output(&format!("LWDISPLAY = {}", if v { "ON" } else { "OFF" }));
                    }
                }
            }
            "CELTSCALE" => {
                use crate::command::ValuePromptCommand;
                let c = ValuePromptCommand::new(
                    "CELTSCALE",
                    "CELTSCALE  new current-object line-type scale:",
                );
                self.command_line.push_info(&c.prompt());
                self.tabs[i].active_cmd = Some(Box::new(c));
            }
            cmd if cmd.starts_with("CELTSCALE ") => {
                let val_str = cmd.trim_start_matches("CELTSCALE").trim();
                if val_str.is_empty() {
                    let v = self.tabs[i]
                        .scene
                        .document
                        .header
                        .current_entity_linetype_scale;
                    self.command_line
                        .push_output(&format!("CELTSCALE = {v:.4}"));
                } else if let Ok(v) = val_str.parse::<f64>() {
                    if v > 0.0 {
                        self.tabs[i]
                            .scene
                            .document
                            .header
                            .current_entity_linetype_scale = v;
                        self.tabs[i].dirty = true;
                        self.command_line
                            .push_output(&format!("CELTSCALE set to {v:.4}"));
                    } else {
                        self.command_line
                            .push_error("CELTSCALE: value must be positive.");
                    }
                } else {
                    self.command_line.push_error("Usage: CELTSCALE [value]");
                }
            }

            // ── SCALETEXT — rescale selected Text/MText entities ─────────────────
            // Usage: SCALETEXT <factor>   e.g. SCALETEXT 2
            //        SCALETEXT H <height>  set absolute height
            cmd if cmd == "SCALETEXT" || cmd.starts_with("SCALETEXT ") => {
                let rest = cmd.trim_start_matches("SCALETEXT").trim();
                let parts: Vec<&str> = rest.split_whitespace().collect();
                let selected_handles: Vec<acadrust::Handle> = self.tabs[i]
                    .scene
                    .selected_entities()
                    .iter()
                    .map(|(h, _)| *h)
                    .collect();
                if selected_handles.is_empty() {
                    self.command_line
                        .push_error("SCALETEXT: select Text/MText entities first.");
                } else {
                    let (use_absolute, value) = match (
                        parts.first().map(|s| s.to_uppercase()).as_deref(),
                        parts.get(1),
                    ) {
                        (Some("H"), Some(v)) => (true, v.parse::<f64>().ok()),
                        (Some(v), None) => (false, v.parse::<f64>().ok()),
                        _ => (false, None),
                    };
                    if let Some(val) = value {
                        if val <= 0.0 {
                            self.command_line
                                .push_error("SCALETEXT: value must be positive.");
                        } else {
                            self.push_undo_snapshot(i, "SCALETEXT");
                            let mut count = 0usize;
                            for sh in &selected_handles {
                                for entity in self.tabs[i].scene.document.entities_mut() {
                                    if entity.common().handle != *sh {
                                        continue;
                                    }
                                    match entity {
                                        acadrust::EntityType::Text(t) => {
                                            t.height =
                                                if use_absolute { val } else { t.height * val };
                                            count += 1;
                                        }
                                        acadrust::EntityType::MText(t) => {
                                            t.height =
                                                if use_absolute { val } else { t.height * val };
                                            count += 1;
                                        }
                                        _ => {}
                                    }
                                    break;
                                }
                            }
                            if count > 0 {
                                self.tabs[i].dirty = true;
                                self.command_line.push_output(&format!(
                                    "SCALETEXT: scaled {count} text entity(ies)."
                                ));
                            } else {
                                self.command_line
                                    .push_error("SCALETEXT: no Text/MText in selection.");
                            }
                        }
                    } else {
                        self.command_line
                            .push_info("Usage: SCALETEXT <factor>  or  SCALETEXT H <height>");
                    }
                }
            }

            _ => return None,
        }
        Some(self.finish_dispatch(cmd))
    }
}
