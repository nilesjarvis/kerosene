use super::http::SEC_ARCHIVES_BASE_URL;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct SecFilingDocument {
    document_type: String,
    filename: String,
    pub(super) text: String,
}

pub(crate) fn sec_filing_document_url(
    cik: u64,
    accession_number: &str,
    primary_document: &str,
) -> Option<String> {
    if cik == 0 {
        return None;
    }

    let accession_digits = accession_digits(accession_number)?;
    let primary_document = primary_document.trim().trim_start_matches('/');
    if accession_digits.is_empty() || !safe_sec_document_path(primary_document) {
        return None;
    }

    Some(format!(
        "{SEC_ARCHIVES_BASE_URL}/{cik}/{accession_digits}/{primary_document}"
    ))
}

pub(super) fn sec_filing_archive_text_url(cik: u64, accession_number: &str) -> Option<String> {
    if cik == 0 {
        return None;
    }
    let accession_digits = accession_digits(accession_number)?;
    let accession_number = accession_number.trim();
    if !safe_sec_accession_number(accession_number) {
        return None;
    }
    Some(format!(
        "{SEC_ARCHIVES_BASE_URL}/{cik}/{accession_digits}/{accession_number}.txt"
    ))
}

fn safe_sec_accession_number(value: &str) -> bool {
    !value.is_empty() && value.chars().all(|c| c.is_ascii_digit() || c == '-')
}

fn safe_sec_document_path(value: &str) -> bool {
    !value.is_empty()
        && !value.contains("..")
        && value
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.' | '/'))
}

fn accession_digits(accession_number: &str) -> Option<String> {
    let digits = accession_number
        .chars()
        .filter(char::is_ascii_digit)
        .collect::<String>();
    (!digits.is_empty()).then_some(digits)
}

pub(super) fn select_filing_summary_documents<'a>(
    documents: &'a [SecFilingDocument],
    primary_document: &str,
    limit: usize,
) -> Vec<&'a SecFilingDocument> {
    let primary = primary_document
        .trim()
        .trim_start_matches('/')
        .to_ascii_lowercase();
    let mut scored = documents
        .iter()
        .filter_map(|item| {
            let name = item.filename.trim().trim_start_matches('/');
            if !filing_document_name_is_safe(name) || !filing_document_is_html(name) {
                return None;
            }
            let score = filing_summary_document_score(&item.document_type, name, &primary);
            (score > 0).then_some((score, item))
        })
        .collect::<Vec<_>>();

    scored.sort_by(|(left_score, left_doc), (right_score, right_doc)| {
        right_score
            .cmp(left_score)
            .then_with(|| left_doc.filename.cmp(&right_doc.filename))
    });

    scored
        .into_iter()
        .take(limit)
        .map(|(_, document)| document)
        .collect::<Vec<_>>()
}

fn filing_summary_document_score(document_type: &str, name: &str, primary_document: &str) -> i32 {
    let document_type = document_type.trim().to_ascii_lowercase();
    let lower = name.to_ascii_lowercase();
    let lower_stem = lower
        .rsplit_once('.')
        .map(|(stem, _)| stem)
        .unwrap_or(lower.as_str());
    if lower.contains("-index")
        || lower.ends_with(".txt")
        || document_type == "xml"
        || lower_stem.starts_with('r')
            && lower_stem[1..]
                .chars()
                .all(|ch| ch.is_ascii_digit() || ch == '.')
    {
        return 0;
    }

    let mut score = 1;
    if document_type.starts_with("ex-99.1") || document_type == "ex99.1" {
        score += 120;
    } else if document_type.starts_with("ex-99") || document_type.starts_with("ex99") {
        score += 100;
    } else if document_type == "8-k" || document_type == "8k" {
        score += 8;
    }
    if lower == primary_document {
        score += 5;
    } else {
        score += 20;
    }
    for (needle, weight) in [
        ("ex99", 90),
        ("exhibit99", 90),
        ("99_1", 85),
        ("991", 80),
        ("press", 55),
        ("pr", 45),
        ("earn", 45),
        ("result", 40),
        ("commentary", 30),
        ("shareholder", 20),
    ] {
        if lower.contains(needle) {
            score += weight;
        }
    }
    score
}

pub(super) fn parse_sec_filing_documents(archive_text: &str) -> Vec<SecFilingDocument> {
    let mut documents = Vec::new();
    let mut cursor = 0;
    while let Some(relative_start) = archive_text[cursor..].find("<DOCUMENT>") {
        let start = cursor + relative_start + "<DOCUMENT>".len();
        let Some(relative_end) = archive_text[start..].find("</DOCUMENT>") else {
            break;
        };
        let end = start + relative_end;
        let block = &archive_text[start..end];
        if let Some(document) = parse_sec_filing_document_block(block) {
            documents.push(document);
        }
        cursor = end + "</DOCUMENT>".len();
    }
    documents
}

fn parse_sec_filing_document_block(block: &str) -> Option<SecFilingDocument> {
    let document_type = sec_document_header_value(block, "TYPE")?;
    let filename = sec_document_header_value(block, "FILENAME")
        .or_else(|| sec_document_header_value(block, "SEQUENCE"))?;
    let text_start = block
        .find("<TEXT>")
        .map(|index| index + "<TEXT>".len())
        .unwrap_or(0);
    let text_end = block[text_start..]
        .find("</TEXT>")
        .map(|index| text_start + index)
        .unwrap_or(block.len());
    let text = block[text_start..text_end].trim().to_string();
    if text.is_empty() || !filing_document_name_is_safe(&filename) {
        return None;
    }

    Some(SecFilingDocument {
        document_type,
        filename,
        text,
    })
}

fn sec_document_header_value(block: &str, name: &str) -> Option<String> {
    let marker = format!("<{name}>");
    let start = block.find(&marker)? + marker.len();
    let value = block[start..].lines().next()?.trim();
    (!value.is_empty()).then(|| value.to_string())
}

pub(super) fn summary_source_document_label(document: &SecFilingDocument) -> String {
    if document.document_type.trim().is_empty() {
        document.filename.clone()
    } else {
        format!("{} {}", document.document_type, document.filename)
    }
}

fn filing_document_name_is_safe(name: &str) -> bool {
    let name = name.trim();
    !name.is_empty()
        && !name.contains("://")
        && !name.contains('\\')
        && !name.contains("..")
        && !name.starts_with('/')
}

fn filing_document_is_html(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    lower.ends_with(".htm") || lower.ends_with(".html")
}

#[cfg(test)]
mod tests;
