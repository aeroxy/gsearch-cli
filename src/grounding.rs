use crate::api::GeminiResponse;

pub fn format_response(response: GeminiResponse) -> String {
    let candidate = match response.response.candidates.first() {
        Some(c) => c,
        None => return "No search results found.".to_string(),
    };

    let content = match &candidate.content {
        Some(c) => c,
        None => return "The request was blocked or returned no content.".to_string(),
    };

    let text = match content.parts.first() {
        Some(p) => &p.text,
        None => return "No search results found.".to_string(),
    };

    let metadata = match &candidate.grounding_metadata {
        Some(m) => m,
        None => return text.clone(),
    };

    let modified_text = text.clone();
    let mut insertions = Vec::new();

    for support in &metadata.grounding_supports {
        let marker = support.grounding_chunk_indices
            .iter()
            .map(|i| format!("[{}]", i + 1))
            .collect::<String>();
        
        insertions.push((support.segment.end_index, marker));
    }

    // Sort insertions by index in descending order
    insertions.sort_by(|a, b| b.0.cmp(&a.0));

    let mut bytes = modified_text.into_bytes();
    for (index, marker) in insertions {
        let marker_bytes = marker.into_bytes();
        let pos = index.min(bytes.len());
        bytes.splice(pos..pos, marker_bytes);
    }

    let mut final_text = String::from_utf8_lossy(&bytes).to_string();

    // Add sources
    if !metadata.grounding_chunks.is_empty() {
        final_text.push_str("\n\nSources:\n");
        for (i, chunk) in metadata.grounding_chunks.iter().enumerate() {
            if let Some(web) = &chunk.web {
                final_text.push_str(&format!("[{}] {} ({})\n", i + 1, web.title, web.uri));
            }
        }
    }

    final_text
}
