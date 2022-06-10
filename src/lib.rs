#[allow(dead_code)]
mod data;

#[allow(dead_code)]
mod errors;

// #[allow(dead_code)]
// mod json_config;

#[allow(dead_code)]
mod parse;

// #[allow(dead_code)]
// mod transform_enrich;

// #[allow(dead_code)]
// mod utils;

use anyhow::anyhow;
use log::trace;
use log4rs;

use anyhow::{Context, Result};
use std::collections::HashMap;
use std::env;
use std::fs;
use std::fs::File;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        let result = 2 + 2;
        assert_eq!(result, 4);
    }
}
