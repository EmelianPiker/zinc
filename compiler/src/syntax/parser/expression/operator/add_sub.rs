//!
//! The addition/subtraction operand parser.
//!

use std::cell::RefCell;
use std::rc::Rc;

use crate::lexical::Lexeme;
use crate::lexical::Location;
use crate::lexical::Symbol;
use crate::lexical::Token;
use crate::lexical::TokenStream;
use crate::syntax::MulDivRemOperatorOperandParser;
use crate::syntax::OperatorExpression;
use crate::syntax::OperatorExpressionBuilder;
use crate::syntax::OperatorExpressionOperator;
use crate::Error;

#[derive(Debug, Clone, Copy)]
pub enum State {
    MulDivRemOperand,
    MulDivRemOperator,
}

impl Default for State {
    fn default() -> Self {
        State::MulDivRemOperand
    }
}

#[derive(Default)]
pub struct Parser {
    state: State,
    builder: OperatorExpressionBuilder,
    operator: Option<(Location, OperatorExpressionOperator)>,
}

impl Parser {
    pub fn parse(mut self, stream: Rc<RefCell<TokenStream>>) -> Result<OperatorExpression, Error> {
        loop {
            match self.state {
                State::MulDivRemOperand => {
                    let rpn = MulDivRemOperatorOperandParser::default().parse(stream.clone())?;
                    self.builder.set_location_if_unset(rpn.location);
                    self.builder.extend_with_expression(rpn);
                    if let Some((location, operator)) = self.operator.take() {
                        self.builder.push_operator(location, operator);
                    }
                    self.state = State::MulDivRemOperator;
                }
                State::MulDivRemOperator => {
                    let peek = stream.borrow_mut().peek();
                    match peek {
                        Some(Ok(Token {
                            lexeme: Lexeme::Symbol(Symbol::Asterisk),
                            location,
                        })) => {
                            stream.borrow_mut().next();
                            self.operator =
                                Some((location, OperatorExpressionOperator::Multiplication));
                            self.state = State::MulDivRemOperand;
                        }
                        Some(Ok(Token {
                            lexeme: Lexeme::Symbol(Symbol::Slash),
                            location,
                        })) => {
                            stream.borrow_mut().next();
                            self.operator = Some((location, OperatorExpressionOperator::Division));
                            self.state = State::MulDivRemOperand;
                        }
                        Some(Ok(Token {
                            lexeme: Lexeme::Symbol(Symbol::Percent),
                            location,
                        })) => {
                            stream.borrow_mut().next();
                            self.operator = Some((location, OperatorExpressionOperator::Remainder));
                            self.state = State::MulDivRemOperand;
                        }
                        _ => return Ok(self.builder.finish()),
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::rc::Rc;

    use super::Parser;
    use crate::lexical;
    use crate::lexical::IntegerLiteral;
    use crate::lexical::Location;
    use crate::lexical::TokenStream;
    use crate::syntax::Literal;
    use crate::syntax::OperatorExpression;
    use crate::syntax::OperatorExpressionElement;
    use crate::syntax::OperatorExpressionObject;
    use crate::syntax::OperatorExpressionOperand;
    use crate::syntax::OperatorExpressionOperator;

    #[test]
    fn ok() {
        let code = r#"42 * 228 "#;

        let expected = OperatorExpression::new(
            Location::new(1, 1),
            vec![
                OperatorExpressionElement::new(
                    Location::new(1, 1),
                    OperatorExpressionObject::Operand(OperatorExpressionOperand::Literal(
                        Literal::new(
                            Location::new(1, 1),
                            lexical::Literal::Integer(IntegerLiteral::decimal("42".to_owned())),
                        ),
                    )),
                ),
                OperatorExpressionElement::new(
                    Location::new(1, 6),
                    OperatorExpressionObject::Operand(OperatorExpressionOperand::Literal(
                        Literal::new(
                            Location::new(1, 6),
                            lexical::Literal::Integer(IntegerLiteral::decimal("228".to_owned())),
                        ),
                    )),
                ),
                OperatorExpressionElement::new(
                    Location::new(1, 4),
                    OperatorExpressionObject::Operator(OperatorExpressionOperator::Multiplication),
                ),
            ],
        );

        let result = Parser::default()
            .parse(Rc::new(RefCell::new(TokenStream::new(code.to_owned()))))
            .expect("Syntax error");

        assert_eq!(expected, result);
    }
}
