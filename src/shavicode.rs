// Wordroute – A word game
// Copyright (C) 2024  Neil Roberts
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program.  If not, see <http://www.gnu.org/licenses/>.

const FIRST_LETTER_SHAVIAN: u32 = '𐑐' as u32;
const LAST_LETTER_SHAVIAN: u32 = '𐑿' as u32;
const N_LETTERS: u32 = LAST_LETTER_SHAVIAN - FIRST_LETTER_SHAVIAN + 1;

pub fn decode_char(ch: char) -> char {
    if ch >= 'A' && ch <= 'Z' {
        char::from_u32(ch as u32 - 'A' as u32 + FIRST_LETTER_SHAVIAN)
            .unwrap()
    } else if ch >= 'a' && ch as u32 <= 'a' as u32 + N_LETTERS - 26 - 1 {
        char::from_u32(ch as u32 - 'a' as u32 + FIRST_LETTER_SHAVIAN + 26)
            .unwrap()
    } else {
        ch
    }
}

pub fn decode_str(s: &str) -> String {
    s.chars().map(decode_char).collect::<String>()
}

pub fn encode_char(ch: char) -> char {
    if (ch as u32) >= FIRST_LETTER_SHAVIAN &&
        (ch as u32) < FIRST_LETTER_SHAVIAN + 26
    {
        char::from((ch as u32 - FIRST_LETTER_SHAVIAN) as u8 + b'A')
    } else if (ch as u32) >= FIRST_LETTER_SHAVIAN + 26 &&
        (ch as u32) <= LAST_LETTER_SHAVIAN
    {
        char::from((ch as u32 - FIRST_LETTER_SHAVIAN - 26) as u8 + b'a')
    } else {
        ch
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn decode_all_letters() {
        assert_eq!(
            &decode_str("ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuv"),
            "𐑐𐑑𐑒𐑓𐑔𐑕𐑖𐑗𐑘𐑙𐑚𐑛𐑜𐑝𐑞𐑟𐑠𐑡𐑢𐑣𐑤𐑥𐑦𐑧𐑨𐑩𐑪𐑫𐑬𐑭𐑮𐑯𐑰𐑱𐑲𐑳𐑴𐑵𐑶𐑷𐑸𐑹𐑺𐑻𐑼𐑽𐑾𐑿"
        );
    }

    #[test]
    fn decode_outside_range() {
        assert_eq!(&decode_str("@Avw"), "@𐑐𐑿w");
    }

    #[test]
    fn encode_all_letters() {
        assert_eq!(
            &"𐑐𐑑𐑒𐑓𐑔𐑕𐑖𐑗𐑘𐑙𐑚𐑛𐑜𐑝𐑞𐑟𐑠𐑡𐑢𐑣𐑤𐑥𐑦𐑧𐑨𐑩𐑪𐑫𐑬𐑭𐑮𐑯𐑰𐑱𐑲𐑳𐑴𐑵𐑶𐑷𐑸𐑹𐑺𐑻𐑼𐑽𐑾𐑿"
                .chars()
                .map(encode_char)
                .collect::<String>(),
            "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuv",
        );
    }

    #[test]
    fn encode_outside_range() {
        assert_eq!(encode_char('\u{1044f}'), '\u{1044f}');
        assert_eq!(encode_char('\u{10480}'), '\u{10480}');
    }
}
