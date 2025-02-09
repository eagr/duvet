// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0

use crate::{
    annotation::{Annotation, AnnotationLevel, AnnotationSet, AnnotationType},
    pattern::Pattern,
    specification::Format,
    Error,
};
use anyhow::{anyhow, Context};
use serde::Deserialize;
use std::{collections::BTreeSet, path::PathBuf};

#[derive(Debug, PartialEq, PartialOrd, Ord, Eq, Hash)]
pub enum SourceFile<'a> {
    Text(Pattern<'a>, PathBuf),
    Spec(PathBuf),
}

impl<'a> SourceFile<'a> {
    pub fn annotations(&self) -> Result<AnnotationSet, Error> {
        let mut annotations = AnnotationSet::new();
        match self {
            Self::Text(pattern, file) => {
                let text = std::fs::read_to_string(file)?;
                pattern
                    .extract(&text, file, &mut annotations)
                    .with_context(|| file.display().to_string())?;
                Ok(annotations)
            }
            Self::Spec(file) => {
                let text = std::fs::read_to_string(&file)?;
                let specs = toml::from_str::<Specs>(&text)?;
                for anno in specs.specs {
                    annotations.insert(anno.into_annotation(file.clone(), &specs.target)?);
                }
                for anno in specs.exceptions {
                    annotations.insert(anno.into_annotation(file.clone(), &specs.target)?);
                }
                for anno in specs.todos {
                    annotations.insert(anno.into_annotation(file.clone(), &specs.target)?);
                }
                Ok(annotations)
            }
        }
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Specs<'a> {
    target: Option<String>,

    #[serde(borrow)]
    #[serde(alias = "spec", default)]
    specs: Vec<Spec<'a>>,

    #[serde(borrow)]
    #[serde(alias = "exception", default)]
    exceptions: Vec<Exception<'a>>,

    #[serde(borrow)]
    #[serde(alias = "TODO", alias = "todo", default)]
    todos: Vec<Todo<'a>>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Spec<'a> {
    target: Option<String>,
    level: Option<&'a str>,
    format: Option<&'a str>,
    quote: &'a str,
}

impl<'a> Spec<'a> {
    fn into_annotation(
        self,
        source: PathBuf,
        default_target: &Option<String>,
    ) -> Result<Annotation, Error> {
        Ok(Annotation {
            anno_line: 0,
            anno_column: 0,
            item_line: 0,
            item_column: 0,
            path: String::new(),
            anno: AnnotationType::Spec,
            target: self
                .target
                .or_else(|| default_target.as_ref().cloned())
                .ok_or_else(|| anyhow!("missing target"))?,
            quote: normalize_quote(self.quote),
            comment: self.quote.to_string(),
            manifest_dir: source.clone(),
            feature: Default::default(),
            tags: Default::default(),
            tracking_issue: Default::default(),
            source,
            level: if let Some(level) = self.level {
                level.parse()?
            } else {
                AnnotationLevel::Auto
            },
            format: if let Some(format) = self.format {
                format.parse()?
            } else {
                Format::Auto
            },
        })
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Exception<'a> {
    target: Option<String>,
    quote: &'a str,
    reason: String,
}

impl<'a> Exception<'a> {
    fn into_annotation(
        self,
        source: PathBuf,
        default_target: &Option<String>,
    ) -> Result<Annotation, Error> {
        Ok(Annotation {
            anno_line: 0,
            anno_column: 0,
            item_line: 0,
            item_column: 0,
            path: String::new(),
            anno: AnnotationType::Exception,
            target: self
                .target
                .or_else(|| default_target.as_ref().cloned())
                .ok_or_else(|| anyhow!("missing target"))?,
            quote: normalize_quote(self.quote),
            comment: self.reason,
            manifest_dir: source.clone(),
            feature: Default::default(),
            tags: Default::default(),
            tracking_issue: Default::default(),
            source,
            level: AnnotationLevel::Auto,
            format: Format::Auto,
        })
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Todo<'a> {
    target: Option<String>,
    quote: &'a str,
    feature: Option<String>,
    #[serde(alias = "tracking-issue")]
    tracking_issue: Option<String>,
    reason: Option<String>,
    #[serde(default)]
    tags: BTreeSet<String>,
}

impl<'a> Todo<'a> {
    fn into_annotation(
        self,
        source: PathBuf,
        default_target: &Option<String>,
    ) -> Result<Annotation, Error> {
        Ok(Annotation {
            anno_line: 0,
            anno_column: 0,
            item_line: 0,
            item_column: 0,
            path: String::new(),
            anno: AnnotationType::Todo,
            target: self
                .target
                .or_else(|| default_target.as_ref().cloned())
                .ok_or_else(|| anyhow!("missing target"))?,
            quote: normalize_quote(self.quote),
            comment: self.reason.unwrap_or_default(),
            manifest_dir: source.clone(),
            source,
            tags: self.tags,
            feature: self.feature.unwrap_or_default(),
            tracking_issue: self.tracking_issue.unwrap_or_default(),
            level: AnnotationLevel::Auto,
            format: Format::Auto,
        })
    }
}

fn normalize_quote(s: &str) -> String {
    s.lines().fold(String::new(), |mut s, l| {
        let l = l.trim();
        if !l.is_empty() && !s.is_empty() {
            s.push(' ');
        }
        s.push_str(l);
        s
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_quote_normalizing() {
        let sample = r"
        A
        B
        C
        ";
        assert_eq!(normalize_quote(sample), "A B C",);
    }

    #[test]
    fn test_quote_normalizing_with_empty_lines() {
        let sample = r"
            A:

            * B

            * C
              D
        ";
        assert_eq!(normalize_quote(sample), "A: * B * C D",);
    }
}
