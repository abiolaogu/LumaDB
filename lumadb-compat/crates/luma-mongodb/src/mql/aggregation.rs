use bson::{Document, Bson};
use luma_protocol_core::{ProtocolError, Result};

#[derive(Debug, Clone)]
pub struct Pipeline {
    pub stages: Vec<Stage>,
}

#[derive(Debug, Clone)]
pub enum Stage {
    Match(Document),
    Project(Document),
    Group(Document),
    Sort(Document),
    Limit(i64),
    Skip(i64),
    Unwind(String), // or Document options
    Lookup(Document),
    AddFields(Document),
    Set(Document), // Alias for AddFields
    ReplaceRoot(Document),
    Count(String),
    Facet(Document),
    Out(String),
    Merge(Document),
    // ...
}

impl Pipeline {
    pub fn parse(pipeline: Vec<Document>) -> Result<Self> {
        let mut stages = Vec::new();
        for doc in pipeline {
            // Each doc should have exactly one key pointing to the stage definition
            if doc.len() != 1 {
                return Err(ProtocolError::Protocol("Stage document must have exactly one key".into()));
            }
            let (key, val) = doc.iter().next().unwrap();
            
            let stage = match key.as_str() {
                "$match" => Stage::Match(val.as_document().cloned().ok_or(ProtocolError::Protocol("$match requires document".into()))?),
                "$project" => Stage::Project(val.as_document().cloned().ok_or(ProtocolError::Protocol("$project requires document".into()))?),
                "$group" => Stage::Group(val.as_document().cloned().ok_or(ProtocolError::Protocol("$group requires document".into()))?),
                "$sort" => Stage::Sort(val.as_document().cloned().ok_or(ProtocolError::Protocol("$sort requires document".into()))?),
                "$limit" => Stage::Limit(val.as_i64().ok_or(ProtocolError::Protocol("$limit requires int".into()))?),
                "$skip" => Stage::Skip(val.as_i64().ok_or(ProtocolError::Protocol("$skip requires int".into()))?),
                "$unwind" => {
                    if let Bson::String(s) = val {
                        Stage::Unwind(s.clone())
                    } else if let Bson::Document(_d) = val {
                        // TODO: Handle options
                        Stage::Unwind("TODO_complex_unwind".into())
                    } else {
                        return Err(ProtocolError::Protocol("$unwind requires string or document".into()));
                    }
                },
                "$lookup" => Stage::Lookup(val.as_document().cloned().ok_or(ProtocolError::Protocol("$lookup requires document".into()))?),
                "$count" => Stage::Count(val.as_str().ok_or(ProtocolError::Protocol("$count requires string".into()))?.to_string()),
                "$out" => Stage::Out(val.as_str().ok_or(ProtocolError::Protocol("$out requires string".into()))?.to_string()),
                _ => return Err(ProtocolError::Protocol(format!("Unknown aggregation stage: {}", key))),
            };
            stages.push(stage);
        }
        Ok(Self { stages })
    }
}
