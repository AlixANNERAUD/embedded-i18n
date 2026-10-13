use proc_macro2::{Ident, TokenStream};
use quote::quote;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Category {
    Zero,
    One,
    Two,
    Few,
    Many,
    Other,
}

impl Category {
    pub const ALL: [Category; 6] = [
        Category::Zero,
        Category::One,
        Category::Two,
        Category::Few,
        Category::Many,
        Category::Other,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Category::Zero => "zero",
            Category::One => "one",
            Category::Two => "two",
            Category::Few => "few",
            Category::Many => "many",
            Category::Other => "other",
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub enum Condition {
    Equals(u64),
    AtMost(u64),
    Range(u64, u64),
    Million,
    SlavicOne,
    SlavicFew,
    SlavicMany,
    PolishMany,
    RomanianFew,
    HundredsRange(u64, u64),
}

impl Condition {
    #[cfg(test)]
    pub fn matches(self, n: u64) -> bool {
        match self {
            Condition::Equals(value) => n == value,
            Condition::AtMost(value) => n <= value,
            Condition::Range(start, end) => (start..=end).contains(&n),
            Condition::Million => n != 0 && n % 1_000_000 == 0,
            Condition::SlavicOne => n % 10 == 1 && n % 100 != 11,
            Condition::SlavicFew => (2..=4).contains(&(n % 10)) && !(12..=14).contains(&(n % 100)),
            Condition::SlavicMany => {
                n % 10 == 0 || (5..=9).contains(&(n % 10)) || (11..=14).contains(&(n % 100))
            }
            Condition::PolishMany => {
                n != 1
                    && (n % 10 <= 1
                        || (5..=9).contains(&(n % 10))
                        || (12..=14).contains(&(n % 100)))
            }
            Condition::RomanianFew => n == 0 || (n != 1 && (1..=19).contains(&(n % 100))),
            Condition::HundredsRange(start, end) => (start..=end).contains(&(n % 100)),
        }
    }

    pub fn tokens(self, n: &Ident) -> TokenStream {
        match self {
            Condition::Equals(value) => quote! { #n == #value },
            Condition::AtMost(value) => quote! { #n <= #value },
            Condition::Range(start, end) => quote! { #n >= #start && #n <= #end },
            Condition::Million => quote! { #n != 0 && #n % 1_000_000 == 0 },
            Condition::SlavicOne => quote! { #n % 10 == 1 && #n % 100 != 11 },
            Condition::SlavicFew => quote! {
                #n % 10 >= 2 && #n % 10 <= 4 && !(#n % 100 >= 12 && #n % 100 <= 14)
            },
            Condition::SlavicMany => quote! {
                #n % 10 == 0 || #n % 10 >= 5 || (#n % 100 >= 11 && #n % 100 <= 14)
            },
            Condition::PolishMany => quote! {
                #n != 1 && (#n % 10 <= 1 || #n % 10 >= 5 || (#n % 100 >= 12 && #n % 100 <= 14))
            },
            Condition::RomanianFew => quote! {
                #n == 0 || (#n != 1 && #n % 100 >= 1 && #n % 100 <= 19)
            },
            Condition::HundredsRange(start, end) => {
                quote! { #n % 100 >= #start && #n % 100 <= #end }
            }
        }
    }
}

pub struct Rules {
    pub categories: &'static [(Category, Condition)],
    pub optional: &'static [Category],
}

impl Rules {
    #[cfg(test)]
    pub fn category(&self, n: u64) -> Category {
        self.categories
            .iter()
            .find(|(_, condition)| condition.matches(n))
            .map(|(category, _)| *category)
            .unwrap_or(Category::Other)
    }

    pub fn required(&self) -> impl Iterator<Item = Category> + '_ {
        self.categories
            .iter()
            .map(|(category, _)| *category)
            .filter(|category| !self.optional.contains(category))
            .chain(std::iter::once(Category::Other))
    }
}

const OTHER: Rules = Rules {
    categories: &[],
    optional: &[],
};

const ONE: Rules = Rules {
    categories: &[(Category::One, Condition::Equals(1))],
    optional: &[],
};

const ZERO_OR_ONE: Rules = Rules {
    categories: &[(Category::One, Condition::AtMost(1))],
    optional: &[],
};

const ONE_MILLION: Rules = Rules {
    categories: &[
        (Category::One, Condition::Equals(1)),
        (Category::Many, Condition::Million),
    ],
    optional: &[Category::Many],
};

const ZERO_OR_ONE_MILLION: Rules = Rules {
    categories: &[
        (Category::One, Condition::AtMost(1)),
        (Category::Many, Condition::Million),
    ],
    optional: &[Category::Many],
};

const EAST_SLAVIC: Rules = Rules {
    categories: &[
        (Category::One, Condition::SlavicOne),
        (Category::Few, Condition::SlavicFew),
        (Category::Many, Condition::SlavicMany),
    ],
    optional: &[],
};

const POLISH: Rules = Rules {
    categories: &[
        (Category::One, Condition::Equals(1)),
        (Category::Few, Condition::SlavicFew),
        (Category::Many, Condition::PolishMany),
    ],
    optional: &[],
};

const WEST_SLAVIC: Rules = Rules {
    categories: &[
        (Category::One, Condition::Equals(1)),
        (Category::Few, Condition::Range(2, 4)),
    ],
    optional: &[],
};

const ROMANIAN: Rules = Rules {
    categories: &[
        (Category::One, Condition::Equals(1)),
        (Category::Few, Condition::RomanianFew),
    ],
    optional: &[],
};

const ARABIC: Rules = Rules {
    categories: &[
        (Category::Zero, Condition::Equals(0)),
        (Category::One, Condition::Equals(1)),
        (Category::Two, Condition::Equals(2)),
        (Category::Few, Condition::HundredsRange(3, 10)),
        (Category::Many, Condition::HundredsRange(11, 99)),
    ],
    optional: &[],
};

const HEBREW: Rules = Rules {
    categories: &[
        (Category::One, Condition::Equals(1)),
        (Category::Two, Condition::Equals(2)),
    ],
    optional: &[],
};

pub fn rules(locale: &str) -> Option<&'static Rules> {
    if locale.eq_ignore_ascii_case("pt-PT") {
        return Some(&ONE_MILLION);
    }
    let language = locale.split('-').next().unwrap_or_default();
    Some(match language {
        "ja" | "zh" | "ko" | "th" | "vi" | "id" | "ms" | "lo" | "my" | "km" => &OTHER,
        "en" | "de" | "nl" | "sv" | "no" | "nb" | "nn" | "da" | "el" | "hu" | "tr" | "fi"
        | "et" | "bg" => &ONE,
        "hi" | "bn" => &ZERO_OR_ONE,
        "it" | "es" | "ca" => &ONE_MILLION,
        "fr" | "pt" => &ZERO_OR_ONE_MILLION,
        "ru" | "uk" | "be" => &EAST_SLAVIC,
        "pl" => &POLISH,
        "cs" | "sk" => &WEST_SLAVIC,
        "ro" => &ROMANIAN,
        "ar" => &ARABIC,
        "he" => &HEBREW,
        _ => return None,
    })
}

#[derive(Clone, Debug, PartialEq)]
pub enum Expression {
    N,
    Number(u64),
    Not(Box<Expression>),
    Binary(Operator, Box<Expression>, Box<Expression>),
    Conditional(Box<Expression>, Box<Expression>, Box<Expression>),
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Operator {
    Or,
    And,
    Equal,
    NotEqual,
    Less,
    LessEqual,
    Greater,
    GreaterEqual,
    Add,
    Subtract,
    Multiply,
    Divide,
    Remainder,
}

impl Operator {
    fn precedence(self) -> u8 {
        match self {
            Operator::Or => 1,
            Operator::And => 2,
            Operator::Equal | Operator::NotEqual => 3,
            Operator::Less | Operator::LessEqual | Operator::Greater | Operator::GreaterEqual => 4,
            Operator::Add | Operator::Subtract => 5,
            Operator::Multiply | Operator::Divide | Operator::Remainder => 6,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Debug)]
enum Token {
    N,
    Number(u64),
    Operator(Operator),
    Not,
    Question,
    Colon,
    Open,
    Close,
}

fn tokenize(source: &str) -> Result<Vec<Token>, String> {
    let mut tokens = Vec::new();
    let bytes = source.as_bytes();
    let mut index = 0;
    while index < bytes.len() {
        let current = bytes[index];
        let next = bytes.get(index + 1).copied();
        let (token, length) = match (current, next) {
            (b' ' | b'\t' | b'\n' | b'\r', _) => {
                index += 1;
                continue;
            }
            (b'n', _) => (Token::N, 1),
            (b'0'..=b'9', _) => {
                let end = bytes[index..]
                    .iter()
                    .position(|byte| !byte.is_ascii_digit())
                    .map_or(bytes.len(), |offset| index + offset);
                let value = source[index..end]
                    .parse()
                    .map_err(|_| format!("number too large in {:?}", source))?;
                (Token::Number(value), end - index)
            }
            (b'|', Some(b'|')) => (Token::Operator(Operator::Or), 2),
            (b'&', Some(b'&')) => (Token::Operator(Operator::And), 2),
            (b'=', Some(b'=')) => (Token::Operator(Operator::Equal), 2),
            (b'!', Some(b'=')) => (Token::Operator(Operator::NotEqual), 2),
            (b'<', Some(b'=')) => (Token::Operator(Operator::LessEqual), 2),
            (b'>', Some(b'=')) => (Token::Operator(Operator::GreaterEqual), 2),
            (b'<', _) => (Token::Operator(Operator::Less), 1),
            (b'>', _) => (Token::Operator(Operator::Greater), 1),
            (b'+', _) => (Token::Operator(Operator::Add), 1),
            (b'-', _) => (Token::Operator(Operator::Subtract), 1),
            (b'*', _) => (Token::Operator(Operator::Multiply), 1),
            (b'/', _) => (Token::Operator(Operator::Divide), 1),
            (b'%', _) => (Token::Operator(Operator::Remainder), 1),
            (b'!', _) => (Token::Not, 1),
            (b'?', _) => (Token::Question, 1),
            (b':', _) => (Token::Colon, 1),
            (b'(', _) => (Token::Open, 1),
            (b')', _) => (Token::Close, 1),
            _ => {
                return Err(format!(
                    "unexpected character {:?} in plural expression {:?}",
                    current as char, source
                ));
            }
        };
        tokens.push(token);
        index += length;
    }
    Ok(tokens)
}

struct Parser<'a> {
    source: &'a str,
    tokens: Vec<Token>,
    position: usize,
}

impl Parser<'_> {
    fn peek(&self) -> Option<Token> {
        self.tokens.get(self.position).copied()
    }

    fn next(&mut self) -> Option<Token> {
        let token = self.peek();
        self.position += 1;
        token
    }

    fn error(&self) -> String {
        format!("invalid plural expression {:?}", self.source)
    }

    fn expect(&mut self, expected: Token) -> Result<(), String> {
        if self.next() == Some(expected) {
            Ok(())
        } else {
            Err(self.error())
        }
    }

    fn conditional(&mut self) -> Result<Expression, String> {
        let condition = self.binary(1)?;
        if self.peek() != Some(Token::Question) {
            return Ok(condition);
        }
        self.next();
        let then = self.conditional()?;
        self.expect(Token::Colon)?;
        let otherwise = self.conditional()?;
        Ok(Expression::Conditional(
            Box::new(condition),
            Box::new(then),
            Box::new(otherwise),
        ))
    }

    fn binary(&mut self, minimum: u8) -> Result<Expression, String> {
        let mut left = self.unary()?;
        while let Some(Token::Operator(operator)) = self.peek() {
            if operator.precedence() < minimum {
                break;
            }
            self.next();
            let right = self.binary(operator.precedence() + 1)?;
            left = Expression::Binary(operator, Box::new(left), Box::new(right));
        }
        Ok(left)
    }

    fn unary(&mut self) -> Result<Expression, String> {
        match self.next() {
            Some(Token::N) => Ok(Expression::N),
            Some(Token::Number(value)) => Ok(Expression::Number(value)),
            Some(Token::Not) => Ok(Expression::Not(Box::new(self.unary()?))),
            Some(Token::Open) => {
                let expression = self.conditional()?;
                self.expect(Token::Close)?;
                Ok(expression)
            }
            _ => Err(self.error()),
        }
    }
}

impl Expression {
    pub fn parse(source: &str) -> Result<Expression, String> {
        let mut parser = Parser {
            source,
            tokens: tokenize(source)?,
            position: 0,
        };
        let expression = parser.conditional()?;
        if parser.position != parser.tokens.len() {
            return Err(parser.error());
        }
        Ok(expression)
    }

    pub fn from_header(header: &str) -> Result<Option<Expression>, String> {
        let Some(line) = header.lines().find_map(|line| {
            let (name, value) = line.split_once(':')?;
            name.trim()
                .eq_ignore_ascii_case("Plural-Forms")
                .then_some(value)
        }) else {
            return Ok(None);
        };
        let expression = line
            .split(';')
            .find_map(|part| part.trim().strip_prefix("plural"))
            .and_then(|rest| rest.trim_start().strip_prefix('='))
            .ok_or_else(|| format!("missing plural= in Plural-Forms {:?}", line.trim()))?;
        Expression::parse(expression.trim()).map(Some)
    }

    pub fn germanic() -> Expression {
        Expression::Binary(
            Operator::NotEqual,
            Box::new(Expression::N),
            Box::new(Expression::Number(1)),
        )
    }

    #[cfg(test)]
    pub fn evaluate(&self, n: u64) -> u64 {
        match self {
            Expression::N => n,
            Expression::Number(value) => *value,
            Expression::Not(operand) => u64::from(operand.evaluate(n) == 0),
            Expression::Conditional(condition, then, otherwise) => {
                if condition.evaluate(n) != 0 {
                    then.evaluate(n)
                } else {
                    otherwise.evaluate(n)
                }
            }
            Expression::Binary(operator, left, right) => {
                let left = left.evaluate(n);
                let right = right.evaluate(n);
                match operator {
                    Operator::Or => u64::from(left != 0 || right != 0),
                    Operator::And => u64::from(left != 0 && right != 0),
                    Operator::Equal => u64::from(left == right),
                    Operator::NotEqual => u64::from(left != right),
                    Operator::Less => u64::from(left < right),
                    Operator::LessEqual => u64::from(left <= right),
                    Operator::Greater => u64::from(left > right),
                    Operator::GreaterEqual => u64::from(left >= right),
                    Operator::Add => left.wrapping_add(right),
                    Operator::Subtract => left.wrapping_sub(right),
                    Operator::Multiply => left.wrapping_mul(right),
                    Operator::Divide => left.checked_div(right).unwrap_or(0),
                    Operator::Remainder => left.checked_rem(right).unwrap_or(0),
                }
            }
        }
    }

    pub fn tokens(&self, n: &Ident) -> TokenStream {
        match self {
            Expression::N => quote! { #n },
            Expression::Number(value) => quote! { #value },
            Expression::Not(operand) => {
                let operand = operand.tokens(n);
                quote! { ((#operand) == 0) as u64 }
            }
            Expression::Conditional(condition, then, otherwise) => {
                let condition = condition.tokens(n);
                let then = then.tokens(n);
                let otherwise = otherwise.tokens(n);
                quote! { if (#condition) != 0 { #then } else { #otherwise } }
            }
            Expression::Binary(operator, left, right) => {
                let left = left.tokens(n);
                let right = right.tokens(n);
                match operator {
                    Operator::Or => quote! { ((#left) != 0 || (#right) != 0) as u64 },
                    Operator::And => quote! { ((#left) != 0 && (#right) != 0) as u64 },
                    Operator::Equal => quote! { ((#left) == (#right)) as u64 },
                    Operator::NotEqual => quote! { ((#left) != (#right)) as u64 },
                    Operator::Less => quote! { ((#left) < (#right)) as u64 },
                    Operator::LessEqual => quote! { ((#left) <= (#right)) as u64 },
                    Operator::Greater => quote! { ((#left) > (#right)) as u64 },
                    Operator::GreaterEqual => quote! { ((#left) >= (#right)) as u64 },
                    Operator::Add => quote! { (#left).wrapping_add(#right) },
                    Operator::Subtract => quote! { (#left).wrapping_sub(#right) },
                    Operator::Multiply => quote! { (#left).wrapping_mul(#right) },
                    Operator::Divide => {
                        quote! { match (#left).checked_div(#right) { Some(value) => value, None => 0 } }
                    }
                    Operator::Remainder => {
                        quote! { match (#left).checked_rem(#right) { Some(value) => value, None => 0 } }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn categories(locale: &str, values: &[u64]) -> Vec<&'static str> {
        let rules = rules(locale).unwrap();
        values.iter().map(|n| rules.category(*n).name()).collect()
    }

    #[test]
    fn english_rules() {
        assert_eq!(
            categories("en", &[0, 1, 2, 11]),
            ["other", "one", "other", "other"]
        );
    }

    #[test]
    fn french_rules() {
        assert_eq!(
            categories("fr-CA", &[0, 1, 2, 1_000_000, 2_000_000, 1_000_001]),
            ["one", "one", "other", "many", "many", "other"]
        );
    }

    #[test]
    fn portuguese_rules_depend_on_region() {
        assert_eq!(categories("pt", &[0, 1]), ["one", "one"]);
        assert_eq!(categories("pt-PT", &[0, 1]), ["other", "one"]);
    }

    #[test]
    fn russian_rules() {
        assert_eq!(
            categories("ru", &[1, 2, 4, 5, 11, 12, 14, 21, 22, 25, 101, 111, 0]),
            [
                "one", "few", "few", "many", "many", "many", "many", "one", "few", "many", "one",
                "many", "many"
            ]
        );
    }

    #[test]
    fn polish_rules() {
        assert_eq!(
            categories("pl", &[0, 1, 2, 5, 12, 21, 22, 101]),
            ["many", "one", "few", "many", "many", "many", "few", "many"]
        );
    }

    #[test]
    fn czech_rules() {
        assert_eq!(
            categories("cs", &[0, 1, 2, 4, 5]),
            ["other", "one", "few", "few", "other"]
        );
    }

    #[test]
    fn romanian_rules() {
        assert_eq!(
            categories("ro", &[0, 1, 2, 19, 20, 101, 119, 120]),
            ["few", "one", "few", "few", "other", "few", "few", "other"]
        );
    }

    #[test]
    fn arabic_rules() {
        assert_eq!(
            categories("ar", &[0, 1, 2, 3, 10, 11, 99, 100, 102, 103]),
            [
                "zero", "one", "two", "few", "few", "many", "many", "other", "other", "few"
            ]
        );
    }

    #[test]
    fn japanese_has_only_other() {
        assert_eq!(categories("ja", &[0, 1, 2]), ["other", "other", "other"]);
    }

    #[test]
    fn unknown_language_has_no_rules() {
        assert!(rules("tlh").is_none());
    }

    #[test]
    fn required_categories() {
        let required = |locale| rules(locale).unwrap().required().collect::<Vec<_>>();
        assert_eq!(required("en"), [Category::One, Category::Other]);
        assert_eq!(required("fr"), [Category::One, Category::Other]);
        assert_eq!(
            required("ru"),
            [
                Category::One,
                Category::Few,
                Category::Many,
                Category::Other
            ]
        );
        assert_eq!(required("ja"), [Category::Other]);
    }

    #[test]
    fn parses_gettext_expressions() {
        let russian = Expression::parse(
            "(n%10==1 && n%100!=11 ? 0 : n%10>=2 && n%10<=4 && (n%100<10 || n%100>=20) ? 1 : 2)",
        )
        .unwrap();
        let values: Vec<u64> = [1, 2, 5, 11, 21, 22, 25, 111]
            .iter()
            .map(|n| russian.evaluate(*n))
            .collect();
        assert_eq!(values, [0, 1, 2, 2, 0, 1, 2, 2]);
        assert_eq!(Expression::parse("n != 1").unwrap().evaluate(1), 0);
        assert_eq!(Expression::parse("n>1").unwrap().evaluate(0), 0);
        assert_eq!(Expression::parse("0").unwrap().evaluate(5), 0);
        assert_eq!(Expression::parse("!(n == 1)").unwrap().evaluate(2), 1);
        assert_eq!(Expression::parse("n - 1 * 2 + 3").unwrap().evaluate(5), 6);
        assert_eq!(Expression::parse("n % 0").unwrap().evaluate(5), 0);
    }

    #[test]
    fn rejects_invalid_expressions() {
        assert!(Expression::parse("n ==").is_err());
        assert!(Expression::parse("(n").is_err());
        assert!(Expression::parse("n ? 1").is_err());
        assert!(Expression::parse("x").is_err());
        assert!(Expression::parse("n 1").is_err());
    }

    #[test]
    fn reads_plural_forms_header() {
        let header =
            "Language: ru\nPlural-Forms: nplurals=3; plural=(n%10==1 && n%100!=11 ? 0 : 1);\n";
        assert_eq!(
            Expression::from_header(header)
                .unwrap()
                .unwrap()
                .evaluate(11),
            1
        );
        assert!(Expression::from_header("Language: fr\n").unwrap().is_none());
        assert!(Expression::from_header("Plural-Forms: nplurals=2;\n").is_err());
    }
}
