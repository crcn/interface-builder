// https://tympanus.net/codrops/css_reference/

use super::ast as pc_ast;
use crate::base::parser::{get_buffer, ParseError};
use crate::js::parser::parse_with_tokenizer as parse_js_with_tokenizer;
use crate::js::ast as js_ast;
use crate::base::tokenizer::{Token, Tokenizer};
use crate::css::parser::parse_with_tokenizer as parse_css_with_tokenizer;

/*

void elements: [ 'area',
  'base',
  'basefont',
  'bgsound',
  'br',
  'col',
  'command',
  'embed',
  'frame',
  'hr',
  'image',
  'img',
  'input',
  'isindex',
  'keygen',
  'link',
  'menuitem',
  'meta',
  'nextid',
  'param',
  'source',
  'track',
  'wbr' ]
*/

pub fn parse<'a>(source: &'a str) -> Result<pc_ast::Node, ParseError> {
  parse_fragment(&mut Tokenizer::new(source))
}

fn parse_fragment<'a>(tokenizer: &mut Tokenizer<'a>) -> Result<pc_ast::Node, ParseError> {
  let mut children: Vec<pc_ast::Node> = vec![];

  while !tokenizer.is_eof() {
    children.push(parse_node(tokenizer)?);
    tokenizer.eat_whitespace();
  }

  if children.len() == 1 {
    Ok(children.pop().unwrap())
  } else {
    Ok(pc_ast::Node::Fragment(pc_ast::Fragment { children }))
  }
}

fn parse_node<'a>(tokenizer: &mut Tokenizer<'a>) -> Result<pc_ast::Node, ParseError> {
  tokenizer.eat_whitespace();
  let token = tokenizer.peek(1)?;
  match token {
    Token::CurlyOpen => { parse_slot(tokenizer) },
    Token::LessThan => { parse_element(tokenizer) },
    Token::HtmlCommentOpen => { 
      tokenizer.next()?; // eat HTML comment open
      let buffer = get_buffer(tokenizer, |tokenizer| {
        let tok = tokenizer.peek(1)?;
        Ok(tok != Token::HtmlCommentClose)
      })?.to_string();
      tokenizer.next()?; // eat -->
      Ok(pc_ast::Node::Comment(pc_ast::ValueObject { value: buffer }))
    },
    Token::BlockOpen => {
      parse_block(tokenizer)
    }
    Token::CloseTag => {
      let start = tokenizer.pos;
      tokenizer.next_expect(Token::CloseTag)?;
      let tag_name = parse_tag_name(tokenizer)?;
      tokenizer.next_expect(Token::GreaterThan)?;

      let message = if is_void_tag_name(tag_name.as_str()) { 
        "Void tag's shouldn't be closed."
      } else {
        "Closing tag doesn't have an open tag."
      };

      Err(ParseError::unexpected(message.to_string(), start, tokenizer.pos))
    }
    _ => {
      let value =  get_buffer(tokenizer, |tokenizer| {
        let tok = tokenizer.peek(1)?;
        Ok(
          tok != Token::CurlyOpen && 
          tok != Token::LessThan && 
          tok != Token::CloseTag && 
          tok != Token::HtmlCommentOpen && 
          tok != Token::BlockOpen && 
          tok != Token::BlockClose
        )
      })?.to_string();

      if value.len() == 0 {
        Err(ParseError::unexpected_token(tokenizer.pos))
      } else {
        Ok(pc_ast::Node::Text(pc_ast::ValueObject { 
          value
        }))
      }
    }
  }
}

fn parse_slot<'a>(tokenizer: &mut Tokenizer<'a>) -> Result<pc_ast::Node, ParseError> {
  let script = parse_slot_script(tokenizer)?;
  Ok(pc_ast::Node::Slot(script))
}

fn parse_slot_script<'a>(tokenizer: &mut Tokenizer<'a>) -> Result<js_ast::Statement, ParseError> {
  let start = tokenizer.pos;
  tokenizer.next_expect(Token::CurlyOpen)?;
  parse_js_with_tokenizer(tokenizer, |token| {
    token != Token::CurlyClose
  })
  .and_then(|script| {
    tokenizer.next_expect(Token::CurlyClose)?;
    Ok(script)
  })
  .or(Err(ParseError::unterminated("Unterminated slot.".to_string(), start, tokenizer.pos)))
}

fn parse_element<'a>(tokenizer: &mut Tokenizer<'a>) -> Result<pc_ast::Node, ParseError> {
  let start = tokenizer.pos;

  tokenizer.next_expect(Token::LessThan)?;
  let tag_name = parse_tag_name(tokenizer)?;

  let attributes = parse_attributes(tokenizer)?;

  if tag_name == "style" {
    parse_next_style_element_parts(attributes, tokenizer, start)
  } else if tag_name == "script" {
    parse_next_script_element_parts(attributes, tokenizer, start)
  } else {
    parse_next_basic_element_parts(tag_name, attributes, tokenizer, start)
  }
}

fn is_void_tag_name<'a>(tag_name: &'a str) -> bool {
  match tag_name {
    "area" | 
    "base" | 
    "basefont" | 
    "bgsound" | 
    "br" | 
    "col" | 
    "command" | 
    "embed" | 
    "frame" |
    "hr" |
    "image" |
    "import" |
    "img" |
    "input" |
    "isindex" |
    "keygen" |
    "link" |
    "menuitem" |
    "meta" |
    "nextid" |
    "param" |
    "source" |
    "track" |
    "wbr" => true,
    _ => false,
  }
}

fn parse_next_basic_element_parts<'a>(tag_name: String, attributes: Vec<pc_ast::Attribute>, tokenizer: &mut Tokenizer<'a>, start: usize) -> Result<pc_ast::Node, ParseError> {
  let mut children: Vec<pc_ast::Node> = vec![];

  tokenizer.eat_whitespace();
  
  match tokenizer.peek(1)? {
    Token::SelfCloseTag => {
      tokenizer.next()?;
    },
    Token::GreaterThan => {
      tokenizer.next()?;
      let end = tokenizer.pos;
      if !is_void_tag_name(tag_name.as_str()) {
        tokenizer.eat_whitespace();
        while !tokenizer.is_eof() && tokenizer.peek(1)? != Token::CloseTag {
          children.push(parse_node(tokenizer)?);
          tokenizer.eat_whitespace();
        }

        parse_close_tag(&tag_name.as_str(), tokenizer, start, end)?;
      }
    },
    _ => {
      return Err(ParseError::unexpected_token(tokenizer.pos))
    }
  }

  let el = pc_ast::Element {
    tag_name,
    attributes,
    children
  };
  Ok(pc_ast::Node::Element(el))
}

fn parse_block<'a>(tokenizer: &mut Tokenizer<'a>) -> Result<pc_ast::Node, ParseError> {
  tokenizer.next_expect(Token::BlockOpen)?;
  let pos = tokenizer.pos;
  let token = tokenizer.next()?; // eat {# or {/
  if let Token::Word(keyword) = token {
    match keyword {
      "if" => parse_if_block(tokenizer),
      _ => {
        Err(ParseError::unexpected_token(pos))
      }
    }
  } else {
    Err(ParseError::unexpected_token(pos))
  }
}

fn parse_if_block<'a>(tokenizer: &mut Tokenizer<'a>) -> Result<pc_ast::Node, ParseError> {
  Ok(pc_ast::Node::Block(pc_ast::Block::Conditional(
    parse_pass_fail_block(tokenizer)?
  )))
}

fn parse_pass_fail_block<'a>(tokenizer: &mut Tokenizer<'a>) -> Result<pc_ast::ConditionalBlock, ParseError> {
  tokenizer.eat_whitespace();
  let condition = parse_js_with_tokenizer(tokenizer, |token| {
    token != Token::CurlyClose
  })?;
  tokenizer.next_expect(Token::CurlyClose)?;
  let node = parse_block_children(tokenizer)?;
  let fail = parse_else_block(tokenizer)?;

  Ok(pc_ast::ConditionalBlock::PassFailBlock(
    pc_ast::PassFailBlock {
      condition,
      node,
      fail,
    }
  ))
}

fn parse_block_children<'a>(tokenizer: &mut Tokenizer<'a>) -> Result<Option<Box<pc_ast::Node>>, ParseError> {

  let mut children = vec![];

  // TODO - we don't really want this since whitespace technically renders. Though, right
  // now it's not handled.
  tokenizer.eat_whitespace();

  while !tokenizer.is_eof() && tokenizer.peek(1)? != Token::BlockClose {
    children.push(parse_node(tokenizer)?);
    tokenizer.eat_whitespace();
  }

  let node = if children.len() == 0 {
    None
  } else if children.len() == 1 {
    Some(Box::new(children.pop().unwrap()))
  } else {
    Some(Box::new(pc_ast::Node::Fragment(pc_ast::Fragment {
      children
    })))
  };

  Ok(node)
}

fn parse_else_block<'a>(tokenizer: &mut Tokenizer<'a>) -> Result<Option<Box<pc_ast::ConditionalBlock>>, ParseError> {
  tokenizer.eat_whitespace();
  tokenizer.next_expect(Token::BlockClose)?;
  tokenizer.eat_whitespace();
  let pos = tokenizer.pos;
  match tokenizer.next()? {
    Token::Word(value) => {
      match value {
        "else" => {
          tokenizer.eat_whitespace();
          let pos = tokenizer.pos;
          match tokenizer.next()? {
            Token::Word(value2) => {
              if value2 == "if" {
                Ok(Some(Box::new(parse_pass_fail_block(tokenizer)?)))
              } else {
                Err(ParseError::unexpected_token(pos))
              }
            },
            Token::CurlyClose => {
              Ok(Some(Box::new(parse_final_condition_block(tokenizer)?)))
            }
            _ => {
              Err(ParseError::unexpected_token(pos))
            }
          }
        },
        _ => {
          Err(ParseError::unexpected_token(pos))
        }
      }
    },
    Token::CurlyClose => {
      Ok(None)
    },
    _ => {
      Err(ParseError::unexpected_token(pos))
    }
  }
}

fn parse_final_condition_block<'a>(tokenizer: &mut Tokenizer<'a>) -> Result<pc_ast::ConditionalBlock, ParseError> {
  let node =  parse_block_children(tokenizer)?;
  tokenizer.next_expect(Token::BlockClose)?;
  tokenizer.next_expect(Token::CurlyClose)?;
  Ok(pc_ast::ConditionalBlock::FinalBlock(pc_ast::FinalBlock {
    node
  }))
}

fn parse_next_style_element_parts<'a>(attributes: Vec<pc_ast::Attribute>, tokenizer: &mut Tokenizer<'a>, start: usize) -> Result<pc_ast::Node, ParseError> {
  tokenizer.next_expect(Token::GreaterThan)?; // eat >
  let end = tokenizer.pos;

  let sheet = parse_css_with_tokenizer(tokenizer, |tokenizer| -> Result<bool, ParseError> {
    Ok(tokenizer.peek(1)? == Token::CloseTag)
  })?;

  // TODO - assert tokens equal these
  parse_close_tag("style", tokenizer, start, end)?;

  Ok(pc_ast::Node::StyleElement(pc_ast::StyleElement {
    attributes,
    sheet,
  }))
}

fn parse_close_tag<'a, 'b>(tag_name: &'a str, tokenizer: &mut Tokenizer<'b>, start: usize, end: usize) -> Result<(), ParseError> {

  let end_tag_name_start = tokenizer.pos;
  
  tokenizer
  .next_expect(Token::CloseTag)
  .or(Err(ParseError::unterminated("Unterminated element.".to_string(), start, end)))?;


  parse_tag_name(tokenizer)
  // TODO - assert tag name
  .and_then(|end_tag_name| {
    if tag_name != end_tag_name {
      Err(ParseError::unterminated(format!("Incorrect closing tag. This should be </{}>.", tag_name), end_tag_name_start, tokenizer.pos))
    } else {
      Ok(())
    }
  })?;

  tokenizer
  .next_expect(Token::GreaterThan)
  .or(Err(ParseError::unterminated("Unterminated element.".to_string(), start, end)))?;

  Ok(())
}

fn parse_next_script_element_parts<'a>(attributes: Vec<pc_ast::Attribute>, tokenizer: &mut Tokenizer<'a>, start: usize) -> Result<pc_ast::Node, ParseError> {
  tokenizer.next_expect(Token::GreaterThan)?; // eat >
  let end = tokenizer.pos;

  get_buffer(tokenizer, |tokenizer| {
    Ok(tokenizer.peek(1)? != Token::CloseTag)
  })?;


  parse_close_tag("script", tokenizer, start, end)?;

  Ok(pc_ast::Node::Element(pc_ast::Element {
    tag_name: "script".to_string(),
    attributes,
    children: vec![],
  }))
}

fn parse_tag_name<'a>(tokenizer: &mut Tokenizer<'a>) -> Result<String, ParseError> {
  Ok(get_buffer(tokenizer, |tokenizer| { Ok(!matches!(tokenizer.peek(1)?, Token::Whitespace | Token::GreaterThan | Token::Equals | Token::SelfCloseTag)) })?.to_string())
}

fn parse_attributes<'a>(tokenizer: &mut Tokenizer<'a>) -> Result<Vec<pc_ast::Attribute>, ParseError> {

  let mut attributes: Vec<pc_ast::Attribute> = vec![];

  loop {
    tokenizer.eat_whitespace();
    match tokenizer.peek(1)? {
      Token::SelfCloseTag | Token::GreaterThan => break,
      _ => {
        attributes.push(parse_attribute(tokenizer)?);
      }
    }
  }

  Ok(attributes)
}

fn parse_attribute<'a>(tokenizer: &mut Tokenizer<'a>) -> Result<pc_ast::Attribute, ParseError> {
  if tokenizer.peek(1)? == Token::CurlyOpen {
    parse_shorthand_attribute(tokenizer)
  } else {
    parse_key_value_attribute(tokenizer)
  }
}

fn parse_shorthand_attribute<'a>(tokenizer: &mut Tokenizer<'a>) -> Result<pc_ast::Attribute, ParseError> {
  let reference = parse_slot_script(tokenizer)?;

  // TODO - expect script to be reference with path.length === 1

  Ok(pc_ast::Attribute::ShorthandAttribute(pc_ast::ShorthandAttribute {
    reference,
  }))
}

fn parse_key_value_attribute<'a>(tokenizer: &mut Tokenizer<'a>) -> Result<pc_ast::Attribute, ParseError> {
  
  let name = parse_tag_name(tokenizer)?;
  let mut value = None;

  if tokenizer.peek(1)? == Token::Equals {
    tokenizer.next()?; // eat =
    value = Some(parse_attribute_value(tokenizer)?);
  }

  Ok(pc_ast::Attribute::KeyValueAttribute(pc_ast::KeyValueAttribute {
    name,
    value
  }))
}

fn parse_attribute_value<'a>(tokenizer: &mut Tokenizer<'a>) -> Result<pc_ast::AttributeValue, ParseError> {
  let pos = tokenizer.pos;
  match tokenizer.peek(1)? {
    Token::SingleQuote | Token::DoubleQuote => parse_string(tokenizer),
    Token::CurlyOpen => parse_attribute_slot(tokenizer),
    _ => Err(ParseError::unexpected_token(pos))
  }
}

fn parse_attribute_slot<'a>(tokenizer: &mut Tokenizer<'a>) -> Result<pc_ast::AttributeValue, ParseError> {
  tokenizer.next_expect(Token::CurlyClose)?;
  let script = parse_slot_script(tokenizer)?;
  Ok(pc_ast::AttributeValue::Slot(script))
}


fn parse_string<'a>(tokenizer: &mut Tokenizer<'a>) -> Result<pc_ast::AttributeValue, ParseError> {
  let start = tokenizer.pos;
  let quote = tokenizer.next()?;


  get_buffer(tokenizer, |tokenizer| { Ok(tokenizer.peek(1)? != quote) })
  .and_then(|value| {
    tokenizer.next_expect(quote)?;
    Ok(value)
  })
  .or(Err(ParseError::unterminated("Unterminated string literal.".to_string(), start, tokenizer.pos)))
  .and_then(|value| {
    Ok(pc_ast::AttributeValue::String(pc_ast::AttributeStringValue { value: value.to_string() }))
  })
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn can_smoke_parse_various_nodes() {

    let source = "
      text
      <!-- comment -->
      <element></element>
      <self-closing-element />
      <element with-attribute />
      <element data-and-an-attribute=\"value\" />
      
      <!-- void tags -->
      <br>
      <import>

      {block}
      
      {#if block}
      {/}

      {#if someting}
        something 
        <div />
      {/else if somethingElse} 
        else if
      {/}

      {#if something}{/else}{/}

      {#if something}a{/else if somethingelse}b{/else}c{/}

      <!-- historically broken stuff -->
      <meta charSet=\"utf-8\" />\n   
      <form action=\"/search/\" autoComplete=\"off\" method=\"get\" role=\"search\">
        <input type=\"search\" id=\"header-search-bar\" name=\"q\" class=\"_2xQx4j6lBnDGQ8QsRnJEJa\" placeholder=\"Search\" value=\"\" />
      </form>\n 
    ";

    parse(source).unwrap();
  }


  #[test]
  fn can_parse_various_nodes() {

    let cases = [

      // text blocks
      "text",

      // comments
      "ab <!--cd-->",

      // slots
      "{ok}",

      // elements
      "<div></div>",
      "<div a b></div>",
      "<div a=\"b\" c></div>",
      "<div a=\"\"></div>",

      "<div a=\"b\" c=\"d\">
        <span>
          c {block} d {block}
        </span>
        <span>
          color {block}
        </span>
      </div>",

      // mixed elements
    ];

    for i in 0..cases.len() {
      let case = cases[i];

      // TODO - strip whitespace
      let expr = parse(case).unwrap();
      assert_eq!(expr.to_string().replace("\n", "").replace(" ", ""), case.replace("\n", "").replace(" ", ""));
    }
  }

  /// 
  /// Error handling
  /// 

  #[test]
  fn displays_error_for_unterminated_element() {
    assert_eq!(parse("<div>"), Err(ParseError::unterminated("Unterminated element.".to_string(), 0, 5)));
  }

  #[test]
  fn displays_error_for_unterminated_style_element() {
    assert_eq!(parse("<style>"), Err(ParseError::unterminated("Unterminated element.".to_string(), 0, 7)));
  }

  #[test]
  fn displays_error_for_unterminated_script_element() {
    assert_eq!(parse("<script>"), Err(ParseError::unterminated("Unterminated element.".to_string(), 0, 8)));
  }


  #[test]
  fn displays_error_for_incorrect_close_tag() {
    assert_eq!(parse("<style></script>"), Err(ParseError::unterminated("Incorrect closing tag. This should be </style>.".to_string(), 7, 15)));
  }

  #[test]
  fn displays_error_for_unterminated_attribute_string() {
    assert_eq!(parse("<div a=\"b>"), Err(ParseError::unterminated("Unterminated string literal.".to_string(), 7, 10)));
  }

  #[test]
  fn displays_error_for_unterminated_slot() {
    assert_eq!(parse("{ab"), Err(ParseError::unterminated("Unterminated slot.".to_string(), 0, 3)));
  }

  #[test]
  fn displays_css_errors() {
    assert_eq!(parse("<style>div { color: red; </style>"), Err(ParseError::unterminated("Unterminated bracket.".to_string(), 11, 27)));
  }

  #[test]
  fn display_error_for_close_tag_without_open() {
    assert_eq!(parse("</div>"), Err(ParseError::unexpected("Closing tag doesn't have an open tag.".to_string(), 0, 6)));
  }


  #[test]
  fn displays_error_if_void_close_tag_present() {
    assert_eq!(parse("</meta>"), Err(ParseError::unexpected("Void tag's shouldn't be closed.".to_string(), 0, 7)));
  }
}
