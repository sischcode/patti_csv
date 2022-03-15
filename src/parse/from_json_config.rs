use std::collections::HashMap;

use super::{config::*, skip_take_file_lines::*, transform_sanitize_token::*};
use crate::{data::imf_value::Value, json_config::parse::dsv::*};

type FileNumLinesFn = Box<dyn Fn() -> usize>;

impl ParserOptLines {
    pub fn from_json_config(sfj: &ParserOptLinesJson, file_num_lines_fn: FileNumLinesFn) -> Self {
        let mut transitizers: Vec<Box<dyn SkipTakeFileLines>> = Vec::new();

        if let Some(num) = sfj.skip_lines_from_start {
            transitizers.push(Box::new(SkipLinesFromStart {
                skip_num_lines: num,
            }));
        }
        if let Some(num) = sfj.skip_lines_from_end {
            transitizers.push(Box::new(SkipLinesFromEnd {
                skip_num_lines: num,
                lines_total: file_num_lines_fn(),
            }));
        }

        let skip_take = (
            sfj.skip_lines_by_startswith.clone(),
            sfj.take_lines_by_startswith.clone(),
        );
        match skip_take {
            (Some(_), Some(take)) | (None, Some(take)) => {
                // the take filter overrules the skip filter.
                for take_when in take {
                    transitizers.push(Box::new(TakeLinesStartingWith {
                        starts_with: take_when,
                    }));
                }
            }
            (Some(skip), None) => {
                for skip_when in skip {
                    transitizers.push(Box::new(SkipLinesStartingWith {
                        starts_with: skip_when,
                    }));
                }
            }
            (None, None) => (),
        }

        if let Some(true) = sfj.skip_empty_lines {
            transitizers.push(Box::new(SkipEmptyLines {}));
        }

        ParserOptLines { transitizers }
    }
}

impl ParserOpts {
    pub fn from_json_config(poj: &ParserOptsJson, file_num_lines_fn: FileNumLinesFn) -> Self {
        Self {
            separator_char: poj.separator_char,
            enclosure_char: poj.enclosure_char,
            lines: match &poj.lines {
                Some(sfj) => Some(ParserOptLines::from_json_config(sfj, file_num_lines_fn)),
                None => None,
            },
            first_line_is_header: poj.first_line_is_header,
        }
    }
}

impl From<TypeColumnsEntryJson> for TypeColumnEntry {
    fn from(tcej: TypeColumnsEntryJson) -> Self {
        Self {
            header: tcej.header.clone(),
            target_type: Value::from(tcej.target_type),
        }
    }
}

fn to_transform_sanitize_tokens(
    sanitizers: Vec<SanitizeColumnOptsJson>,
) -> Vec<Box<dyn TransformSanitizeToken>> {
    let mut transform_sanitize = Vec::<Box<dyn TransformSanitizeToken>>::new();
    for mut col_sanitizer in sanitizers {
        match col_sanitizer {
            SanitizeColumnOptsJson::Trim { spec } => match spec {
                TrimOptsJson::Leading => transform_sanitize.push(Box::new(TrimLeading {})),
                TrimOptsJson::Trailing => transform_sanitize.push(Box::new(TrimTrailing {})),
                TrimOptsJson::All => transform_sanitize.push(Box::new(TrimAll {})),
            },
            SanitizeColumnOptsJson::Casing { spec } => match spec {
                CasingOptsJson::ToLower => transform_sanitize.push(Box::new(ToLowercase {})),
                CasingOptsJson::ToUpper => transform_sanitize.push(Box::new(ToUppercase {})),
            },
            SanitizeColumnOptsJson::Eradicate { mut spec } => {
                for eradicate in spec.iter_mut() {
                    transform_sanitize.push(Box::new(Eradicate {
                        eradicate: std::mem::take(eradicate),
                    }));
                }
            }
            SanitizeColumnOptsJson::Replace { mut spec } => {
                for entry in spec.iter_mut() {
                    transform_sanitize.push(Box::new(ReplaceWith {
                        from: std::mem::take(&mut entry.from),
                        to: std::mem::take(&mut entry.to),
                    }));
                }
            }
            SanitizeColumnOptsJson::RegexTake { ref mut spec } => {
                transform_sanitize.push(Box::new(RegexTake::new(std::mem::take(spec))))
            }
        }
    }
    transform_sanitize
}

impl DsvParserConfig {
    pub fn from_json_config(
        cfg_json: DsvParserConfigJson,
        file_num_lines_fn: FileNumLinesFn,
    ) -> Self {
        let parser_opts = ParserOpts::from_json_config(&cfg_json.parser_opts, file_num_lines_fn);

        // Create the column sanitizers aka string transformation. Global, i.e. None and based on index, i.e. Some(index),
        let sanitize_columns = cfg_json.sanitize_columns.map_or(None, |ref mut vcsej| {
            let mut csm: HashMap<Option<usize>, TransformSanitizeTokens> = HashMap::new();
            for csej in vcsej.iter_mut() {
                csm.insert(
                    csej.idx, // CAUTION: This CAN be None, and that is valid! That's the key for our global sanitizers!
                    TransformSanitizeTokens {
                        transitizers: to_transform_sanitize_tokens(std::mem::take(
                            &mut csej.sanitizers,
                        )),
                    },
                );
            }
            Some(csm)
        });

        // Create the type information hashMap
        let type_columns: Vec<TypeColumnEntry> = cfg_json
            .type_columns
            .into_iter()
            .map(|e| TypeColumnEntry::from(e))
            .collect();

        DsvParserConfig {
            parser_opts,
            sanitize_columns,
            type_columns,
        }
    }
}
