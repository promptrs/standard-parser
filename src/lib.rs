#[allow(warnings)]
mod bindings;

use bindings::promptrs::core::util::ToolCall;
use bindings::{Delims, Guest, Response};
use serde::{Deserialize, Deserializer};
use std::collections::HashMap;
use winnow::combinator::{empty, opt, repeat, seq};
use winnow::error::ParserError;
use winnow::token::take_until;
use winnow::{Parser, Result};

struct Component;

impl Guest for Component {
	fn parse(response: String, delims: Option<Delims>) -> Response {
		parse(&mut response.as_str(), delims).unwrap_or(Response {
			reasoning: None,
			content: response,
			tool_calls: vec![],
		})
	}
}

fn parse(input: &mut &str, delims: Option<Delims>) -> Result<Response> {
	let Some(Delims {
		reasoning,
		tool_call: delims,
	}) = delims
	else {
		return Ok(Response {
			reasoning: None,
			content: input.to_string(),
			tool_calls: vec![],
		});
	};
	let Some(rdelims) = reasoning else {
		return seq!(Response {
			reasoning: empty.value(None),
			content: take_until(0.., delims.0.as_str()).map(|s: &str| s.into()),
			tool_calls: repeat(0.., between(&delims)).map(parse_args)
		})
		.parse_next(input);
	};

	seq!(Response {
		reasoning: opt(between(&rdelims)).map(|s: Option<&str>| s.map(|s| s.into())),
		content: take_until(0.., delims.0.as_str()).map(|s: &str| s.into()),
		tool_calls: repeat(0.., between(&delims)).map(parse_args)
	})
	.parse_next(input)
}

fn between<'s, E: ParserError<&'s str>>(
	(start, end): &(String, String),
) -> impl Parser<&'s str, &'s str, E> {
	|input: &mut &'s str| {
		let (mut start, mut end) = (start.as_str(), end.as_str());
		_ = take_until(0.., start).parse_next(input)?;
		_ = start.parse_next(input)?;
		let between = take_until(0.., end).parse_next(input)?;
		_ = end.parse_next(input)?;
		Ok(between)
	}
}

fn parse_args(list: Vec<&str>) -> Vec<ToolCall> {
	list.into_iter()
		.map(|tc| {
			serde_json::from_str(tc).unwrap_or(ToolCallDef {
				name: "".into(),
				arguments: vec![],
			})
		})
		.map(|ToolCallDef { name, arguments }| ToolCall { name, arguments })
		.collect()
}

#[derive(Deserialize)]
struct ToolCallDef {
	name: String,
	#[serde(deserialize_with = "deserialize_args")]
	arguments: Vec<(String, String)>,
}

fn deserialize_args<'de, D: Deserializer<'de>>(
	deserializer: D,
) -> Result<Vec<(String, String)>, D::Error> {
	let map = HashMap::<String, String>::deserialize(deserializer)?;
	Ok(map.into_iter().collect())
}

bindings::export!(Component with_types_in bindings);
