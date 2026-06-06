use tracing::{debug, warn};

#[derive(Debug, Clone, PartialEq)]
pub struct InlineComment {
    pub file_path: String,
    pub line: u32,
    pub body: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ReviewVerdict {
    Approve,
    RequestChanges,
    Comment,
}

#[derive(Debug, Clone, PartialEq)]
pub struct StructuredReview {
    pub verdict: ReviewVerdict,
    pub summary: String,
    pub inline_comments: Vec<InlineComment>,
}

pub struct ReviewParser;

impl ReviewParser {
    pub fn parse(llm_output: &str) -> StructuredReview {
        let mut verdict = ReviewVerdict::Comment;
        let mut summary = String::new();
        let mut inline_comments = Vec::new();
        
        let mut current_file: Option<String> = None;
        let mut current_line: Option<u32> = None;
        let mut current_comment = String::new();
        let mut in_summary = false;
        
        for line in llm_output.lines() {
            let trimmed = line.trim();
            
            // Parse verdict
            if trimmed.to_uppercase().starts_with("VERDICT:") {
                let v = trimmed[8..].trim().to_uppercase();
                verdict = match v.as_str() {
                    "APPROVE" | "APPROVED" => ReviewVerdict::Approve,
                    "REQUEST_CHANGES" | "REQUEST CHANGES" | "CHANGES_REQUESTED" => ReviewVerdict::RequestChanges,
                    _ => ReviewVerdict::Comment,
                };
                continue;
            }
            
            // Parse summary section
            if trimmed.to_uppercase().starts_with("SUMMARY:") {
                in_summary = true;
                summary.push_str(trimmed[8..].trim());
                summary.push('\n');
                continue;
            }
            
            if in_summary {
                if trimmed.to_uppercase().starts_with("FILE:") || 
                   trimmed.to_uppercase().starts_with("INLINE") ||
                   trimmed.to_uppercase().starts_with("COMMENT") {
                    in_summary = false;
                } else {
                    summary.push_str(line);
                    summary.push('\n');
                    continue;
                }
            }
            
            // Parse inline comments
            if trimmed.to_uppercase().starts_with("FILE:") {
                // Save previous comment if exists
                if let (Some(file), Some(line_num)) = (current_file.take(), current_line.take()) {
                    if !current_comment.trim().is_empty() {
                        inline_comments.push(InlineComment {
                            file_path: file,
                            line: line_num,
                            body: current_comment.trim().to_string(),
                        });
                    }
                }
                current_comment.clear();
                current_file = Some(trimmed[5..].trim().to_string());
                continue;
            }
            
            if trimmed.to_uppercase().starts_with("LINE:") {
                let line_str = trimmed[5..].trim();
                current_line = line_str.parse().ok();
                if current_line.is_none() {
                    warn!("Failed to parse line number: {}", line_str);
                }
                continue;
            }
            
            if trimmed.to_uppercase().starts_with("COMMENT:") {
                current_comment.push_str(trimmed[8..].trim());
                current_comment.push('\n');
                continue;
            }
            
            // If we're building a comment, append the line
            if current_file.is_some() && current_line.is_some() {
                current_comment.push_str(line);
                current_comment.push('\n');
            }
        }
        
        // Save last comment
        if let (Some(file), Some(line_num)) = (current_file, current_line) {
            if !current_comment.trim().is_empty() {
                inline_comments.push(InlineComment {
                    file_path: file,
                    line: line_num,
                    body: current_comment.trim().to_string(),
                });
            }
        }
        
        debug!(
            "Parsed review: verdict={:?}, {} inline comments",
            verdict,
            inline_comments.len()
        );
        
        StructuredReview {
            verdict,
            summary: summary.trim().to_string(),
            inline_comments,
        }
    }
    
    pub fn format_simple_review(structured: &StructuredReview) -> String {
        let mut output = String::new();
        
        output.push_str(&format!("## Review Summary\n\n{}", structured.summary));
        
        if !structured.inline_comments.is_empty() {
            output.push_str("\n\n## Inline Comments\n");
            for comment in &structured.inline_comments {
                output.push_str(&format!(
                    "\n**{}:{}**\n{}\n",
                    comment.file_path,
                    comment.line,
                    comment.body
                ));
            }
        }
        
        output.push_str(&format!("\n\n**Verdict:** {:?}", structured.verdict));
        
        output
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_verdict_approve() {
        let output = "VERDICT: APPROVE\nSUMMARY: Looks good!";
        let review = ReviewParser::parse(output);
        assert_eq!(review.verdict, ReviewVerdict::Approve);
        assert_eq!(review.summary, "Looks good!");
    }

    #[test]
    fn test_parse_verdict_request_changes() {
        let output = "VERDICT: REQUEST_CHANGES\nSUMMARY: Needs work";
        let review = ReviewParser::parse(output);
        assert_eq!(review.verdict, ReviewVerdict::RequestChanges);
    }

    #[test]
    fn test_parse_inline_comments() {
        let output = r#"VERDICT: COMMENT
SUMMARY: Some issues found

FILE: src/main.rs
LINE: 42
COMMENT: This could panic
Consider using unwrap_or_default

FILE: src/lib.rs
LINE: 10
COMMENT: Good documentation
"#;
        
        let review = ReviewParser::parse(output);
        assert_eq!(review.inline_comments.len(), 2);
        assert_eq!(review.inline_comments[0].file_path, "src/main.rs");
        assert_eq!(review.inline_comments[0].line, 42);
        assert!(review.inline_comments[0].body.contains("unwrap_or_default"));
    }

    #[test]
    fn test_parse_no_inline() {
        let output = "VERDICT: APPROVE\nSUMMARY: All good";
        let review = ReviewParser::parse(output);
        assert!(review.inline_comments.is_empty());
    }
}
