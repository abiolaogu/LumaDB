
use nom::{
    bytes::complete::{tag, take_while, take_while1, is_not},
    character::complete::{alpha1, alphanumeric1, char, multispace0, digit1},
    combinator::{map, map_res, opt, recognize, value},
    sequence::{delimited, pair, preceded, separated_pair, tuple},
    branch::alt,
    multi::{separated_list0, separated_list1},
    IResult,
};
use crate::ir::{Expression, Operation, BinaryOperator, AggregateFunction};
use crate::Value;

#[derive(Debug, Clone)]
pub struct PromQLParser;

impl PromQLParser {
    pub fn parse(query: &str) -> Result<Operation, String> {
        match parse_expr(query) {
            Ok((_, op)) => Ok(op),
            Err(e) => Err(format!("Parse error: {}", e)),
        }
    }
}

// Basic grammar subset
// metric_name{label="value"}

fn parse_expr(input: &str) -> IResult<&str, Operation> {
    // For now, only support VectorSelectors and simple Aggregations
    alt((
        parse_aggregation,
        parse_vector_selector,
    ))(input)
}

fn parse_vector_selector(input: &str) -> IResult<&str, Operation> {
    map(
        tuple((
            parse_identifier,
            opt(parse_label_matchers),
            // opt(parse_range) // TODO
        )),
        |(name, labels)| {
            let mut filter_expr = None;
            
            // Convert labels to filter expression (ANDed)
            if let Some(lbls) = labels {
                for (key, val) in lbls {
                    let eq = Expression::BinaryOp {
                        op: BinaryOperator::Eq,
                        left: Box::new(Expression::Column(key.to_string())),
                        right: Box::new(Expression::Literal(Value::Text(val.to_string()))),
                    };
                    
                    filter_expr = match filter_expr {
                         Some(expr) => Some(Expression::BinaryOp {
                             op: BinaryOperator::And,
                             left: Box::new(expr),
                             right: Box::new(eq),
                         }),
                         None => Some(eq),
                    };
                }
            }

            // Implicit filter on _measurement
            let meas_eq = Expression::BinaryOp {
                op: BinaryOperator::Eq,
                left: Box::new(Expression::Column("_measurement".to_string())),
                right: Box::new(Expression::Literal(Value::Text(name.to_string()))),
            };

             filter_expr = match filter_expr {
                 Some(expr) => Some(Expression::BinaryOp {
                     op: BinaryOperator::And,
                     left: Box::new(expr),
                     right: Box::new(meas_eq),
                 }),
                 None => Some(meas_eq),
            };

            Operation::Scan {
                table: "timeseries".to_string(), // Virtual table
                alias: None,
                filter: filter_expr.map(Box::new),
                columns: vec![], // All columns
            }
        }
    )(input)
}

fn parse_label_matchers(input: &str) -> IResult<&str, Vec<(&str, &str)>> {
    delimited(
        char('{'),
        separated_list0(
            char(','),
            parse_label_matcher
        ),
        char('}')
    )(input)
}

fn parse_label_matcher(input: &str) -> IResult<&str, (&str, &str)> {
    separated_pair(
        parse_identifier,
        delimited(multispace0, tag("="), multispace0), // Only Eq for now
        parse_string_literal
    )(input)
}

fn parse_identifier(input: &str) -> IResult<&str, &str> {
    recognize(
        pair(
            alt((alpha1, tag("_"))),
            take_while(|c: char| c.is_alphanumeric() || c == '_')
        )
    )(input)
}

fn parse_string_literal(input: &str) -> IResult<&str, &str> {
    delimited(
        char('"'),
        take_while(|c: char| c != '"'),
        char('"')
    )(input)
}

fn parse_aggregation(input: &str) -> IResult<&str, Operation> {
    // sum(...) by (...)
    // Simplified: sum(selector)
    map(
        tuple((
            tag("sum"),
            multispace0,
            delimited(char('('), parse_vector_selector, char(')')),
            // Optional BY
        )),
        |(_, _, inner_op)| {
            Operation::Aggregate {
                 group_by: vec![], // TODO
                 aggregates: vec![
                     AggregateFunction {
                         name: "sum".to_string(),
                         args: vec![], // Implicitly on value
                         alias: Some("value".to_string()),
                     }
                 ]
            }
        }
    )(input)
}
