use crate::error::{PostcssError, Result};
use std::borrow::Cow;
use tokenizer::{tokenize, Token, TokenType};

pub struct Root<'a> {
  pub children: Vec<RuleOrAtRuleOrDecl<'a>>,
  pub(crate) start: usize,
  pub(crate) end: usize,
}

pub enum RuleOrAtRuleOrDecl<'a> {
  Rule(Rule<'a>),
  AtRule(AtRule<'a>),
  Declaration(Declaration<'a>),
}

// enum AtRuleOrDeclaration<'a> {
//   Declaration(Declaration<'a>),
//   AtRule(AtRule<'a>),
// }

pub struct Rule<'a> {
  pub children: Vec<RuleOrAtRuleOrDecl<'a>>,
  pub start: usize,
  pub end: usize,
  pub selector: Cow<'a, str>,
}

pub struct Declaration<'a> {
  pub prop: Cow<'a, str>,
  pub value: Cow<'a, str>,
  pub(crate) start: usize,
  pub(crate) end: usize,
}

pub struct AtRule<'a> {
  pub params: Cow<'a, str>,
  pub name: Cow<'a, str>,
  pub(crate) start: usize,
  pub(crate) end: usize,
  pub children: Vec<RuleOrAtRuleOrDecl<'a>>,
}

pub struct Parser<'a> {
  lexer: Vec<Token>,
  source: &'a str,
  pos: usize,
  cursor: usize,
}

impl<'a> Parser<'a> {
  pub fn new(input: &'a str) -> Self {
    Self {
      lexer: tokenize(input),
      source: input,
      pos: 0,
      cursor: 0,
    }
  }

  pub fn parse(mut self) -> Result<Root<'a>> {
    // self.parse_element();
    let mut children: Vec<RuleOrAtRuleOrDecl> = vec![];
    while let Some(syntax) = self.peek() {
      match syntax {
        TokenType::Space => {
          self.bump();
        }
        TokenType::AtWord => {
          children.push(RuleOrAtRuleOrDecl::AtRule(self.parse_at_rule()?));
        }
        TokenType::Comment => {
          self.parse_comment();
        }
        _ => {
          children.push(RuleOrAtRuleOrDecl::Rule(self.parse_rule()?));
        }
      };
    }
    Ok(Root {
      children,
      start: 0,
      end: self.pos,
    })
  }

  #[inline]
  pub fn parse_comment(&mut self) {
    self.bump();
  }

  #[inline]
  pub fn parse_rule(&mut self) -> Result<Rule<'a>> {
    let start = self.pos;
    if let Some(kind) = self.peek() {
      match kind {
        TokenType::OpenCurly => {
          return Ok(Rule {
            selector: Cow::Borrowed(""),
            children: self.parse_curly_block(false)?,
            start,
            end: self.pos,
          });
        }
        _ => {
          self.parse_component()?;
          let mut selector_end = self.pos;
          loop {
            match self.peek() {
              Some(kind) => match kind {
                TokenType::OpenCurly => {
                  return Ok(Rule {
                    selector: Cow::Borrowed(&self.source[start..selector_end]),
                    children: self.parse_curly_block(false)?,
                    start,
                    end: self.pos,
                  });
                }
                TokenType::Space | TokenType::Comment => {
                  self.bump();
                }
                _ => {
                  self.parse_component()?;
                  selector_end = self.pos;
                }
              },
              None => {
                return Err(PostcssError::ParseError(
                  format!(r#"expected {} found <EOF>"#, "{",),
                  self.pos,
                  self.pos,
                ));
              }
            }
          }
        }
      }
    } else {
      unimplemented!("should parse a Rule")
    }
  }

  // https://drafts.csswg.org/css-syntax/#component-value-diagram
  #[inline]
  /// return bump token is trivial
  fn parse_component(&mut self) -> Result<bool> {
    // self.start_node(TokenType::Component);
    if let Some(kind) = self.peek() {
      match kind {
        TokenType::OpenCurly => {
          self.parse_curly_block_in_component()?;
        }
        TokenType::OpenParentheses => {
          // println!("parse open parentheses");
          self.parse_parentheses_block()?;
        }
        TokenType::OpenSquare => {
          self.parse_square_block()?;
        }
        _ => {
          // println!("need to bump {:?} from parse component", self.peek());
          return Ok(matches!(
            self.bump().0,
            TokenType::Space | TokenType::Comment
          ));
        }
      }
    } else {
      return Err(PostcssError::ParseError(
        "expected token found <EOF>".to_string(),
        self.pos,
        self.pos,
      ));
    }
    Ok(false)
    // self.finish_node();
  }

  fn parse_parentheses_block(&mut self) -> Result<()> {
    self.bump(); // bump (
    loop {
      match self.peek() {
        Some(kind) => match kind {
          TokenType::CloseParentheses => {
            self.bump();
            break;
          }
          _ => {
            self.parse_component()?;
          }
        },
        None => {
          // TODO: error handle
          return Err(PostcssError::ParseError(
            "expected ) found <EOF>".to_string(),
            self.pos,
            self.pos,
          ));
        }
      }
    }
    Ok(())
  }

  fn parse_square_block(&mut self) -> Result<()> {
    self.bump(); // bump [
    loop {
      match self.peek() {
        Some(kind) => match kind {
          TokenType::CloseSquare => {
            self.bump();
            break;
          }
          _ => {
            self.parse_component()?;
          }
        },
        None => {
          return Err(PostcssError::ParseError(
            "expected ] found <EOF>".to_string(),
            self.pos,
            self.pos,
          ));
        }
      }
    }
    Ok(())
  }

  fn parse_curly_block_in_component(&mut self) -> Result<()> {
    self.bump(); // bump {
    loop {
      match self.peek() {
        Some(kind) => match kind {
          TokenType::CloseCurly => {
            self.bump();
            break;
          }
          _ => {
            self.parse_component()?;
          }
        },
        None => {
          return Err(PostcssError::ParseError(
            format!("expected {} found <EOF>", "}"),
            self.pos,
            self.pos,
          ));
        }
      }
    }
    Ok(())
  }

  fn parse_curly_block(&mut self, rule: bool) -> Result<Vec<RuleOrAtRuleOrDecl<'a>>> {
    use TokenType::*;
    // println!("parse curlyblock");
    let mut ret: Vec<RuleOrAtRuleOrDecl> = vec![];
    self.bump(); // bump {
    self.skip_whitespace_comment();
    loop {
      match self.peek() {
        Some(kind) => match kind {
          Semicolon => {
            self.bump();
          }
          AtWord => ret.push(RuleOrAtRuleOrDecl::AtRule(self.parse_at_rule()?)),
          Space | Comment => {
            self.bump();
          }
          CloseCurly => {
            self.bump();
            // println!("finish close curly");
            break;
          }
          _ => {
            let mut checkpoint = self.cursor;
            let first_token = self.lexer[checkpoint];
            if matches!(first_token.0, Word)
              || potential_id_token(
                self.source[first_token.1..first_token.1 + 1]
                  .chars()
                  .next()
                  .unwrap(),
              )
            {
              checkpoint += 1;
              while checkpoint < self.lexer.len()
                && matches!(self.lexer[checkpoint].0, Comment | Space)
              {
                checkpoint += 1;
              }
              let second_token = if checkpoint < self.lexer.len() {
                self.lexer[checkpoint]
              } else {
                panic!("expected token found <EOF>");
              };
              match second_token.0 {
                Colon => ret.push(RuleOrAtRuleOrDecl::Declaration(self.parse_declaration()?)),
                _ => {
                  ret.push(RuleOrAtRuleOrDecl::Rule(self.parse_rule()?));
                }
              }
            } else {
              ret.push(RuleOrAtRuleOrDecl::Rule(self.parse_rule()?));
            }
            // if rule {
            //   // println!("parse rule -->");
            //   ret.push(RuleOrAtRuleOrDecl::Rule(self.parse_rule()?));
            // } else {
            //   // println!("parse declaration");
            //   ret.push(RuleOrAtRuleOrDecl::Declaration(self.parse_declaration()?));
            // }
          }
        },
        None => {
          return Err(PostcssError::ParseError(
            "expected close curly found <EOF>".to_string(),
            self.pos,
            self.pos,
          ));
        }
      }
    }
    Ok(ret)
  }

  fn parse_declaration(&mut self) -> Result<Declaration<'a>> {
    use TokenType::*;
    match self.peek() {
      Some(Word) => {}
      Some(other) => {
        return Err(PostcssError::ParseError(
          format!("expected token word, found `{}`", other),
          self.pos,
          self.lexer[self.cursor].2,
        ));
      }
      None => {
        return Err(PostcssError::ParseError(
          "expected token word, found <EOF>".to_string(),
          self.pos,
          self.pos,
        ));
      }
    }
    // if !matches!(self.peek(), Some(Word)) {
    // }
    let Token(_, prop_start, prop_end) = self.bump();
    let prop = Cow::Borrowed(&self.source[prop_start..prop_end]);
    self.skip_whitespace_comment();
    match self.peek() {
      Some(TokenType::Colon) => {}
      Some(other) => {
        return Err(PostcssError::ParseError(
          format!("expected `:`, found `{}`", other),
          self.pos,
          self.lexer[self.cursor].2,
          // self.lexer.peek().unwrap().2,
        ));
      }
      None => {
        return Err(PostcssError::ParseError(
          "expected token word, found <EOF>".to_string(),
          self.pos,
          self.pos,
        ));
      }
    }
    self.bump();
    self.skip_whitespace_comment();
    let mut has_finish = false;
    let mut value: Cow<'a, str> = Cow::default();
    let value_start = self.pos;
    let mut value_end = self.pos;
    while let Some(kind) = self.peek() {
      match kind {
        CloseCurly | Semicolon => {
          has_finish = true;
          value = Cow::Borrowed(&self.source[value_start..value_end]);
          break;
        }
        Space => {
          self.bump();
        }
        _ => {
          // println!("parse the component");
          self.parse_component()?;
          value_end = self.pos;
        }
      }
    }
    if !has_finish {
      // value.end = value_end;
      value = Cow::Borrowed(&self.source[value_start..value_end]);
    }
    let end = if matches!(self.peek(), Some(Semicolon)) {
      // self.lexer.peek().unwrap().2
      self.lexer[self.cursor].2
    } else {
      value_end
    };
    Ok(Declaration {
      start: prop_start,
      end,
      prop,
      value,
    })
  }

  pub fn parse_at_rule(&mut self) -> Result<AtRule<'a>> {
    // TODO: should parse declaration inside a at_rule
    use TokenType::*;
    let start = self.pos;
    let Token(_, word_start, word_end) = self.bump(); // bump atWord
    self.skip_whitespace_comment();
    let mut children = vec![];
    let params_start = self.pos;
    let mut params_end = self.pos;
    while let Some(kind) = self.peek() {
      match kind {
        OpenCurly => {
          //   self.finish_node(); finish params
          children = self.parse_curly_block(true)?;
          break;
        }
        Semicolon => {
          //   self.finish_node();
          self.bump();
          break;
        }
        CloseCurly => {
          break;
        }
        _ => {
          if !self.parse_component()? {
            params_end = self.pos;
          }
        }
      }
    }
    Ok(AtRule {
      params: Cow::Borrowed(&self.source[params_start..params_end]),
      name: Cow::Borrowed(&self.source[word_start + 1..word_end]),
      start,
      end: self.pos,
      children,
    })
  }

  #[inline]
  pub fn skip_whitespace_comment(&mut self) {
    while matches!(self.peek(), Some(TokenType::Space | TokenType::Comment)) {
      self.bump();
    }
  }

  #[inline]
  pub fn peek(&mut self) -> Option<TokenType> {
    if self.cursor < self.lexer.len() {
      Some(self.lexer[self.cursor].0)
    } else {
      None
    }
    // self.lexer.peek().map(|token| token.0)
    // self.lexer[self.lexer]
  }

  pub fn bump(&mut self) -> Token {
    let token = self.lexer[self.cursor];
    self.cursor += 1;
    self.pos = token.2;
    token
  }
}

fn potential_id_token(content: char) -> bool {
  match content {
    'a'..='z' => true,
    'A'..='Z' => true,
    '-' => true,
    '_' => true,
    '\\' => true,
    _ => false,
  }
}
