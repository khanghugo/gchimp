use common::take_until_unbalanced;
use nom::{
    bytes::complete::{tag, take_till},
    character::complete::multispace0,
    combinator::map_parser,
    sequence::{preceded, terminated, tuple},
};

use crate::types::SResult;

/// This takes in a &str so remember to convert
// this is the same function in Qc module
pub fn between_braces<'a, T>(
    f: impl FnMut(&'a str) -> SResult<'a, T>,
) -> impl FnMut(&'a str) -> SResult<'a, T> {
    map_parser(
        preceded(
            tuple((multispace0, tag("{"), multispace0)),
            terminated(
                take_until_unbalanced('{', '}'),
                // tag("hello"),
                tuple((tag("}"), multispace0)),
            ),
        ),
        f,
    )
}

pub fn quoted_text(i: &str) -> SResult<&str> {
    terminated(preceded(tag("\""), take_till(|c| c == '\"')), tag("\""))(i)
}
