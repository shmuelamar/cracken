use crate::MAX_WORD_SIZE;
use regex::Regex;

#[derive(Debug, PartialEq)]
pub enum MaskOp {
    Char(char),
    BuiltinCharset(char),
    CustomCharset(usize),
    Wordlist(usize),
}

/// parses `mask` string into the operations it means
pub fn parse_mask(mask: &str) -> Result<Vec<MaskOp>, &'static str> {
    if !is_valid_mask(mask) {
        return Err("Invalid mask");
    }

    let mut mask_ops = vec![];
    let mut chars = mask.chars();
    let mut next = chars.next();

    while next.is_some() {
        let ch = next.unwrap();
        match ch {
            // 1. escaped char (like \?)
            '\\' => mask_ops.push(MaskOp::Char(chars.next().unwrap())),
            // 2. charsets (like ?d)
            '?' => {
                let next_chr = chars.next().unwrap();

                // 2.1 custom charset
                if next_chr.is_digit(10) {
                    mask_ops.push(MaskOp::CustomCharset(((next_chr as u8) - b'1') as usize))

                // 2.2 wordlist
                } else if next_chr == 'w' {
                    let idx = chars.next().unwrap();
                    mask_ops.push(MaskOp::Wordlist(((idx as u8) - b'1') as usize));

                // 2.3 builtin charset
                } else {
                    mask_ops.push(MaskOp::BuiltinCharset(next_chr))
                }
            }
            // 3. single char
            _ => mask_ops.push(MaskOp::Char(ch)),
        }
        next = chars.next();
    }
    Ok(mask_ops)
}

/// returns true iff the mask is valid
fn is_valid_mask(mask: &str) -> bool {
    lazy_static! {
        static ref RE: Regex = Regex::new(
            format!(
                r"^(\?[ludsab1-9]|\?w[1-9]|\\.|[^?\\]){{1,{}}}$",
                MAX_WORD_SIZE
            )
            .as_str()
        )
        .unwrap();
    }
    RE.is_match(mask)
}

#[cfg(test)]
mod tests {
    use super::{is_valid_mask, parse_mask, MaskOp};

    #[test]
    fn test_is_valid_mask() {
        let valid_masks = vec![
            "?d?d?d?d",
            "?l?u?a?b?s",
            "abc?l?u?a?b?sdef?1?2?3",
            "?a?b\\?",
        ];
        for mask in valid_masks {
            assert!(is_valid_mask(mask));
        }

        let invalid_masks = vec!["", "?", "?x", "??", "?"];
        for mask in invalid_masks {
            assert!(!is_valid_mask(mask));
        }
    }

    #[test]
    fn test_parse_mask() {
        let valid_masks = vec![
            (
                "?d?d",
                vec![MaskOp::BuiltinCharset('d'), MaskOp::BuiltinCharset('d')],
            ),
            (
                "?l?u?a?b?s",
                vec![
                    MaskOp::BuiltinCharset('l'),
                    MaskOp::BuiltinCharset('u'),
                    MaskOp::BuiltinCharset('a'),
                    MaskOp::BuiltinCharset('b'),
                    MaskOp::BuiltinCharset('s'),
                ],
            ),
            (
                "a ?ld?1?2\\?a",
                vec![
                    MaskOp::Char('a'),
                    MaskOp::Char(' '),
                    MaskOp::BuiltinCharset('l'),
                    MaskOp::Char('d'),
                    MaskOp::CustomCharset(0),
                    MaskOp::CustomCharset(1),
                    MaskOp::Char('?'),
                    MaskOp::Char('a'),
                ],
            ),
        ];

        for (mask, expected) in valid_masks {
            let mask_ops = parse_mask(mask).unwrap();
            assert_eq!(mask_ops, expected);
        }
    }
}
