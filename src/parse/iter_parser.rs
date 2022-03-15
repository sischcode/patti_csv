// use log::trace;
use std::io::Read;

use crate::data::imf::IMF;
use crate::data::imf_column::Column;
use crate::data::imf_value::Value;
use crate::errors::{PattiCsvError, Result};
use crate::parse::dsv::line_tokenizer::DelimitedLineTokenizer;

use super::{
    config::DsvParserConfig,
    line_tokenizer::{DelimitedLineTokenizerIterator, DelimitedLineTokenizerStats},
    parser_common::{build_imf_skeleton, build_imf_skeleton_w_header, sanitize_tokenizer_iter_res},
};

pub struct DsvIterParser<'rd, 'cfg, R>
where
    R: Read,
{
    pub config: &'cfg DsvParserConfig,
    pub dlt_iter: DelimitedLineTokenizerIterator<'rd, 'cfg, R>,
}

impl<'rd, 'cfg, R: Read> DsvIterParser<'rd, 'cfg, R> {
    pub fn new(config: &'cfg DsvParserConfig, input: &'rd mut R) -> Self {
        Self {
            config,
            dlt_iter: DelimitedLineTokenizer::custom(
                &config.parser_opts.lines,
                input,
                config.parser_opts.separator_char,
                config.parser_opts.enclosure_char,
            )
            .into_iter(),
        }
    }
}

pub struct DsvIterParserIterator<'rd, 'cfg, R: Read> {
    parser: DsvIterParser<'rd, 'cfg, R>,
    col_layout_template: Option<IMF>,
}

impl<'rd, 'cfg, R: Read> Iterator for DsvIterParserIterator<'rd, 'cfg, R> {
    type Item = Result<(usize, IMF, DelimitedLineTokenizerStats)>;

    fn next(&mut self) -> Option<Self::Item> {
        let (dlt_iter_line_num, dlt_iter_res) = self.parser.dlt_iter.next()?; // shortcuts when iteration is over!
        let (dlt_iter_res_vec, dlt_iter_res_stats) = match dlt_iter_res {
            Ok((v, s)) => (v, s),
            Err(e) => return Some(Err(e)),
        };

        let mut imf_ret = IMF::new();
        match dlt_iter_res_stats.is_at_header_line() {
            true => {
                // Special case for first line. We create a skeleton with or without supplied headers
                if self.parser.config.parser_opts.first_line_is_header {
                    self.col_layout_template = match build_imf_skeleton_w_header(
                        &dlt_iter_res_vec,
                        &self.parser.config.type_columns,
                    ) {
                        Ok(v) => Some(v),
                        Err(e) => return Some(Err(e)),
                    }
                } else {
                    self.col_layout_template =
                        Some(build_imf_skeleton(&self.parser.config.type_columns));
                }

                dlt_iter_res_vec.into_iter().enumerate().for_each(|(i, v)| {
                    let mut new_col = Column::new(Value::string_default(), v.clone(), i);
                    new_col.push_data(v.into());
                    imf_ret.add_col(new_col);
                });
            }
            false => {
                imf_ret = match self.col_layout_template.clone() {
                    Some(v) => v,
                    None => {
                        return Some(Err(PattiCsvError::Generic {
                            msg: "Error! No structure template available, but expected one.".into(),
                        }))
                    }
                };

                let sanitized_tokens = match sanitize_tokenizer_iter_res(
                    self.parser.config,
                    (dlt_iter_line_num, dlt_iter_res_vec),
                ) {
                    Ok(v) => v,
                    Err(e) => return Some(Err(e)),
                };

                let mut col_iter = imf_ret.columns.iter_mut().enumerate();
                while let Some((i, col)) = col_iter.next() {
                    let curr_token = sanitized_tokens.get(i).unwrap();
                    col.push(
                        match Value::from_string_with_templ(curr_token.clone(), &col.imf_type) {
                            Ok(v) => v,
                            Err(e) => return Some(Err(e)),
                        },
                    );
                }
            }
        }
        Some(Ok((0, imf_ret, dlt_iter_res_stats)))
    }
}

impl<'rd, 'cfg, R: Read> IntoIterator for DsvIterParser<'rd, 'cfg, R> {
    type Item = Result<(usize, IMF, DelimitedLineTokenizerStats)>;
    type IntoIter = DsvIterParserIterator<'rd, 'cfg, R>;

    fn into_iter(self) -> Self::IntoIter {
        DsvIterParserIterator {
            parser: self,
            col_layout_template: None,
        }
    }
}
