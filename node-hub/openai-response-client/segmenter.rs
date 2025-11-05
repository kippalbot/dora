/// Segmenter for streaming text that intelligently buffers chunks into meaningful segments.
///
/// The segmenter is designed for real-time text-to-speech applications where sending
/// individual characters or words would overwhelm the TTS system. It buffers incoming
/// text chunks and emits complete segments based on punctuation boundaries.
///
/// # Example
/// ```
/// let mut segmenter = StreamSegmenter::new(10);
/// assert_eq!(segmenter.add_chunk("Hello"), None);
/// assert_eq!(segmenter.add_chunk(" world."), Some("Hello world.".to_string()));
/// ```
pub struct StreamSegmenter {
    buffer: String,
    word_count: usize,
    max_words_without_punctuation: usize,
    max_chars_without_punctuation: usize, // For Chinese text
}

impl StreamSegmenter {
    pub fn new(max_words: usize) -> Self {
        Self {
            buffer: String::new(),
            word_count: 0,
            max_words_without_punctuation: max_words,
            max_chars_without_punctuation: 10, // Strict 10 Chinese character limit for TTS
        }
    }

    /// Add a text chunk to the buffer and return a segment if one is ready.
    ///
    /// The method buffers the incoming chunk and checks if a meaningful segment
    /// can be emitted based on punctuation marks or word count limits.
    ///
    /// # Arguments
    /// * `chunk` - A text chunk received from the streaming API
    ///
    /// # Returns
    /// * `Some(String)` - A complete segment ready for TTS processing
    /// * `None` - No segment ready yet, more buffering needed
    pub fn add_chunk(&mut self, chunk: &str) -> Option<String> {
        // Clean markdown from the chunk before adding to buffer
        let cleaned_chunk = self.clean_markdown(chunk);
        self.buffer.push_str(&cleaned_chunk);

        // Count words in the current buffer
        self.word_count = self.buffer.split_whitespace().count();

        // Check if we should emit a segment
        if self.should_emit_segment() {
            self.emit_segment()
        } else {
            None
        }
    }

    /// Check if we should emit a segment
    fn should_emit_segment(&self) -> bool {
        if self.buffer.is_empty() {
            return false;
        }

        // Check for meaningful punctuation (Chinese and English)
        let punctuation_marks = [
            '。', '！', '？', '；', '：', // Chinese
            '.', '!', '?', ';', ':', // English
            '，', ',',  // Comma (both Chinese and English)
            '\n', // Newline
        ];

        // Check if buffer ends with punctuation
        let has_punctuation = self.buffer.chars().any(|c| punctuation_marks.contains(&c));

        // Count Chinese characters (for Chinese text segmentation)
        let chinese_char_count = self
            .buffer
            .chars()
            .filter(|c| self.is_chinese_char(*c))
            .count();

        // Emit if:
        // 1. We have punctuation, OR
        // 2. We have reached max words without punctuation (for English), OR
        // 3. We have reached max Chinese characters without punctuation
        has_punctuation
            || self.word_count >= self.max_words_without_punctuation
            || chinese_char_count >= self.max_chars_without_punctuation
    }

    /// Check if a character is Chinese
    fn is_chinese_char(&self, c: char) -> bool {
        // Unicode ranges for Chinese characters
        match c {
            // CJK Unified Ideographs
            '\u{4e00}'..='\u{9fff}' => true,
            // CJK Unified Ideographs Extension A
            '\u{3400}'..='\u{4dbf}' => true,
            // CJK Unified Ideographs Extension B
            '\u{20000}'..='\u{2a6df}' => true,
            // CJK Unified Ideographs Extension C
            '\u{2a700}'..='\u{2b73f}' => true,
            // CJK Unified Ideographs Extension D
            '\u{2b740}'..='\u{2b81f}' => true,
            _ => false,
        }
    }

    /// Emit the current buffer as a segment
    fn emit_segment(&mut self) -> Option<String> {
        if self.buffer.is_empty() {
            return None;
        }

        // Find the best split point
        let split_point = self.find_split_point();

        if split_point > 0 {
            // Take the segment up to the split point
            let segment = self.buffer[..split_point].to_string();

            // Keep the rest in the buffer
            self.buffer = self.buffer[split_point..].trim_start().to_string();
            self.word_count = self.buffer.split_whitespace().count();

            Some(segment)
        } else if self.word_count >= self.max_words_without_punctuation {
            // No punctuation found but we've reached max words
            // Emit the entire buffer
            let segment = self.buffer.clone();
            self.buffer.clear();
            self.word_count = 0;
            Some(segment)
        } else {
            // Check if we need to emit due to Chinese character count
            let chinese_char_count = self
                .buffer
                .chars()
                .filter(|c| self.is_chinese_char(*c))
                .count();

            if chinese_char_count >= self.max_chars_without_punctuation {
                // Too many Chinese characters without punctuation
                let segment = self.buffer.clone();
                self.buffer.clear();
                self.word_count = 0;
                Some(segment)
            } else {
                None
            }
        }
    }

    /// Find the best point to split the buffer
    fn find_split_point(&self) -> usize {
        // Count Chinese characters to enforce strict limit
        let chinese_char_count = self
            .buffer
            .chars()
            .filter(|c| self.is_chinese_char(*c))
            .count();

        // Priority punctuation for splitting (sentence endings first)
        let sentence_endings = ['。', '！', '？', '.', '!', '?'];
        let clause_endings = ['；', '：', ';', ':'];
        let soft_breaks = ['，', ',', '\n'];

        // For Chinese text, be more aggressive with segmentation
        if chinese_char_count > 0 {
            // If we're approaching the limit, look for ANY punctuation to split at
            if chinese_char_count >= 8 {
                // Try sentence endings first
                if let Some(pos) = self.find_punctuation_before_limit(&sentence_endings, 10) {
                    return pos + self.char_len_at(pos);
                }
                // Then clause endings
                if let Some(pos) = self.find_punctuation_before_limit(&clause_endings, 10) {
                    return pos + self.char_len_at(pos);
                }
                // Then soft breaks (comma)
                if let Some(pos) = self.find_punctuation_before_limit(&soft_breaks, 10) {
                    return pos + self.char_len_at(pos);
                }
            }
        }

        // For English or mixed text, use the original logic
        // Try to find sentence ending first
        if let Some(pos) = self.find_last_punctuation(&sentence_endings) {
            return pos + self.char_len_at(pos);
        }

        // Then try clause endings
        if let Some(pos) = self.find_last_punctuation(&clause_endings) {
            return pos + self.char_len_at(pos);
        }

        // Finally try soft breaks if we have enough words
        if self.word_count >= 5 || chinese_char_count >= 5 {
            if let Some(pos) = self.find_last_punctuation(&soft_breaks) {
                return pos + self.char_len_at(pos);
            }
        }

        0
    }

    /// Find the position of the first punctuation character before the limit
    fn find_punctuation_before_limit(&self, chars: &[char], limit: usize) -> Option<usize> {
        let mut count = 0;
        for (idx, ch) in self.buffer.char_indices() {
            count += 1;
            if chars.contains(&ch) {
                return Some(idx);
            }
            if count >= limit {
                break;
            }
        }
        None
    }

    /// Find the last occurrence of any punctuation from the provided list
    fn find_last_punctuation(&self, chars: &[char]) -> Option<usize> {
        self.buffer
            .char_indices()
            .rev()
            .find(|(_, ch)| chars.contains(ch))
            .map(|(idx, _)| idx)
    }

    /// Get the length (in bytes) of the character at the provided index
    fn char_len_at(&self, idx: usize) -> usize {
        self.buffer[idx..]
            .chars()
            .next()
            .map(|c| c.len_utf8())
            .unwrap_or(0)
    }

    /// Clean markdown elements from the text chunk for better TTS compatibility
    fn clean_markdown(&self, chunk: &str) -> String {
        let mut result = chunk.to_string();

        // List markers
        let list_markers = ['-', '*', '+'];
        if result.len() > 2
            && result.chars().nth(1) == Some(' ')
            && list_markers.contains(&result.chars().next().unwrap_or_default())
        {
            result = result.chars().skip(2).collect();
        }

        // Numeric lists like "1. Step" or "12. Step"
        if result.len() > 3 {
            let mut chars = result.chars();
            let first = chars.next().unwrap_or_default();
            let second = chars.next().unwrap_or_default();
            if first.is_numeric() && second == '.' && chars.next() == Some(' ') {
                result = result.chars().skip(3).collect();
            } else if first.is_numeric()
                && second.is_numeric()
                && chars.next() == Some('.')
                && chars.next() == Some(' ')
            {
                result = result.chars().skip(4).collect();
            }
        }

        // Remove bold/italic markers
        result = result.replace("**", "");
        result = result.replace("__", "");
        result = result.replace('*', "");
        result = result.replace('_', "");

        // Remove headers and quotes
        if result.starts_with('#') {
            result = result.trim_start_matches('#').trim_start().to_string();
        }
        if result.starts_with('>') {
            result = result.trim_start_matches('>').trim_start().to_string();
        }

        // Remove inline code markers
        result = result.replace('`', "");

        // Replace links [text](url) with just text
        if result.contains('[')
            && result.contains(']')
            && result.contains('(')
            && result.contains(')')
        {
            let mut cleaned = String::new();
            let mut chars = result.chars().peekable();
            while let Some(ch) = chars.next() {
                if ch == '[' {
                    let mut text = String::new();
                    while let Some(&next_ch) = chars.peek() {
                        if next_ch == ']' {
                            chars.next(); // consume ']'
                            break;
                        }
                        text.push(next_ch);
                        chars.next();
                    }

                    // Skip over "(...)" part if present
                    if let Some(&next_ch) = chars.peek() {
                        if next_ch == '(' {
                            while let Some(&inner_ch) = chars.peek() {
                                chars.next();
                                if inner_ch == ')' {
                                    break;
                                }
                            }
                        }
                    }

                    cleaned.push_str(&text);
                } else {
                    cleaned.push(ch);
                }
            }
            result = cleaned;
        }

        // Replace hyphens with spaces in Chinese context (e.g., "2根-土豆" → "2根 土豆")
        // This helps TTS pronounce it more naturally
        result = result.replace('-', " ");

        // Collapse multiple consecutive spaces into single space
        // Preserve leading/trailing spaces since we're processing chunks
        while result.contains("  ") {
            result = result.replace("  ", " ");
        }

        result
    }

    /// Force emit any remaining buffered content.
    ///
    /// This should be called when the stream ends to ensure no text is lost.
    /// After flushing, the buffer is cleared and ready for new content.
    ///
    /// # Returns
    /// * `Some(String)` - The remaining buffered text
    /// * `None` - Buffer was already empty
    pub fn flush(&mut self) -> Option<String> {
        if self.buffer.is_empty() {
            None
        } else {
            let segment = self.buffer.clone();
            self.buffer.clear();
            self.word_count = 0;
            Some(segment)
        }
    }
}
