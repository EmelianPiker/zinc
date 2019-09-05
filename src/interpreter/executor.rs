//!
//! The interpreter executor.
//!

use std::cell::RefCell;
use std::convert::TryFrom;
use std::rc::Rc;

use crate::interpreter::Element;
use crate::interpreter::ElementError;
use crate::interpreter::Error;
use crate::interpreter::Place;
use crate::interpreter::Scope;
use crate::interpreter::Value;
use crate::interpreter::Warning;
use crate::lexical::Literal;
use crate::syntax::BlockExpression;
use crate::syntax::Expression;
use crate::syntax::OperatorExpression;
use crate::syntax::OperatorExpressionObject;
use crate::syntax::OperatorExpressionOperand;
use crate::syntax::OperatorExpressionOperator;
use crate::syntax::Statement;
use crate::syntax::TypeVariant;

#[derive(Default)]
pub struct Executor {
    stack: Vec<Element>,
    scope: Rc<RefCell<Scope>>,
}

impl Executor {
    pub fn new(scope: Scope) -> Self {
        Self {
            stack: Default::default(),
            scope: Rc::new(RefCell::new(scope)),
        }
    }

    pub fn execute(&mut self, statement: Statement) -> Result<(), Error> {
        log::trace!("Statement              : {}", statement);

        match statement {
            Statement::Debug(debug) => {
                let result = self.evaluate(debug.expression)?;
                log::info!("{}", result);
            }
            Statement::Let(r#let) => {
                let value = self.evaluate(r#let.expression)?;
                let value = if let Some(r#type) = r#let.r#type {
                    match (value, r#type.variant) {
                        (value @ Value::Void, TypeVariant::Void) => value,
                        (value @ Value::Boolean(_), TypeVariant::Bool) => value,
                        (Value::Integer(mut integer), type_variant) => {
                            integer = integer.cast(type_variant).map_err(|error| {
                                Error::Element(r#type.location, ElementError::Value(error))
                            })?;
                            Value::Integer(integer)
                        }
                        (value, type_variant) => {
                            return Err(Error::LetDeclarationInvalidType(
                                r#let.location,
                                value,
                                type_variant,
                            ))
                        }
                    }
                } else {
                    value
                };

                let location = r#let.identifier.location;
                let place = Place::new(r#let.identifier, value, r#let.is_mutable);
                if let Some(warning) = self.scope.borrow_mut().declare_variable(place) {
                    log::warn!("{}", Warning::Scope(location, warning));
                }
            }
            Statement::Require(require) => match self.evaluate(require.expression)? {
                Value::Boolean(ref boolean) if boolean.is_true() => {
                    log::info!("require {} passed", require.id);
                }
                Value::Boolean(ref boolean) if boolean.is_false() => {
                    return Err(Error::RequireFailed(require.location, require.id))
                }
                value => {
                    return Err(Error::RequireExpectedBooleanExpression(
                        require.location,
                        require.id,
                        value,
                    ))
                }
            },
            Statement::Loop(r#loop) => {
                log::trace!("Loop statement         : {}", r#loop);

                let location = r#loop.location;

                let range_start = match Value::try_from(r#loop.range_start) {
                    Ok(Value::Integer(integer)) => integer,
                    Ok(value) => {
                        return Err(Error::Element(
                            location,
                            ElementError::ExpectedIntegerValue(
                                OperatorExpressionOperator::Range,
                                Element::Value(value),
                            ),
                        ))
                    }
                    Err(error) => return Err(Error::Element(location, ElementError::Value(error))),
                };
                let range_end = match Value::try_from(r#loop.range_end) {
                    Ok(Value::Integer(integer)) => integer,
                    Ok(value) => {
                        return Err(Error::Element(
                            location,
                            ElementError::ExpectedIntegerValue(
                                OperatorExpressionOperator::Range,
                                Element::Value(value),
                            ),
                        ))
                    }
                    Err(error) => return Err(Error::Element(location, ElementError::Value(error))),
                };

                let mut index = if range_start.has_the_same_type_as(&range_end) {
                    range_start
                } else {
                    range_start
                        .cast(TypeVariant::uint(range_end.bitlength()))
                        .map_err(|error| Error::Element(location, ElementError::Value(error)))?
                };

                if index
                    .greater(&range_end)
                    .map_err(|error| Error::Element(location, ElementError::Value(error)))?
                    .is_true()
                {
                    return Err(Error::LoopRangeInvalid(location, index, range_end));
                }

                let mut warning_logged = false;
                while index
                    .lesser(&range_end)
                    .map_err(|error| Error::Element(location, ElementError::Value(error)))?
                    .is_true()
                {
                    let mut scope = Scope::new(Some(self.scope.clone()));
                    let place = Place::new(
                        r#loop.index_identifier.clone(),
                        Value::Integer(index.clone()),
                        false,
                    );

                    if let Some(warning) = scope.declare_variable(place) {
                        if !warning_logged {
                            log::warn!("{}", Warning::Scope(location, warning));
                            warning_logged = true;
                        }
                    }

                    let mut executor = Executor::new(scope);
                    for statement in r#loop.block.statements.clone() {
                        executor.execute(statement)?;
                    }
                    if let Some(expression) = r#loop.block.expression.clone() {
                        executor.evaluate(*expression)?;
                    }
                    index = index
                        .inc()
                        .map_err(|error| Error::Element(location, ElementError::Value(error)))?;
                }
            }
            Statement::Expression(expression) => {
                self.evaluate(expression)?;
            }
        }
        Ok(())
    }

    pub fn evaluate(&mut self, expression: Expression) -> Result<Value, Error> {
        match expression {
            Expression::Operator(expression) => self.evaluate_operator(expression),
            Expression::Block(expression) => self.evaluate_block(expression),
        }
    }

    pub fn evaluate_operator(&mut self, expression: OperatorExpression) -> Result<Value, Error> {
        log::trace!("Operator expression    : {}", expression);

        for expression_element in expression.into_iter() {
            match expression_element.object {
                OperatorExpressionObject::Operand(operand) => {
                    let element = match operand {
                        OperatorExpressionOperand::Literal(Literal::Void) => {
                            Element::Value(Value::Void)
                        }
                        OperatorExpressionOperand::Literal(Literal::Boolean(literal)) => {
                            Element::Value(Value::from(literal))
                        }
                        OperatorExpressionOperand::Literal(Literal::Integer(literal)) => {
                            let location = expression_element.token.location;
                            Element::Value(Value::try_from(literal).map_err(|error| {
                                Error::Element(location, ElementError::Value(error))
                            })?)
                        }
                        OperatorExpressionOperand::Literal(literal @ Literal::String(..)) => {
                            return Err(Error::LiteralIsNotSupported(
                                expression_element.token.location,
                                literal,
                            ));
                        }
                        OperatorExpressionOperand::Type(r#type) => Element::Type(r#type),
                        OperatorExpressionOperand::Identifier(identifier) => {
                            let location = expression_element.token.location;
                            self.scope
                                .borrow()
                                .get_variable(&identifier)
                                .map(Element::Place)
                                .map_err(|error| Error::Scope(location, error))?
                        }
                        OperatorExpressionOperand::Block(block) => {
                            Element::Value(self.evaluate_block(block)?)
                        }
                    };
                    self.stack.push(element);
                }
                OperatorExpressionObject::Operator(OperatorExpressionOperator::Assignment) => {
                    let (element_2, element_1) = (
                        self.stack.pop().expect("Option state bug"),
                        self.stack.pop().expect("Option state bug"),
                    );
                    self.scope
                        .borrow_mut()
                        .update_variable(element_1.assign(element_2).map_err(|error| {
                            Error::Element(expression_element.token.location, error)
                        })?)
                        .map_err(|error| Error::Scope(expression_element.token.location, error))?;
                    self.stack.push(Element::Value(Value::Void));
                }
                OperatorExpressionObject::Operator(OperatorExpressionOperator::Range) => {
                    panic!("The range operator cannot be used in expressions (yet)")
                }
                OperatorExpressionObject::Operator(OperatorExpressionOperator::Or) => {
                    let (element_2, element_1) = (
                        self.stack.pop().expect("Option state bug"),
                        self.stack.pop().expect("Option state bug"),
                    );
                    self.stack.push(element_1.or(element_2).map_err(|error| {
                        Error::Element(expression_element.token.location, error)
                    })?);
                }
                OperatorExpressionObject::Operator(OperatorExpressionOperator::Xor) => {
                    let (element_2, element_1) = (
                        self.stack.pop().expect("Option state bug"),
                        self.stack.pop().expect("Option state bug"),
                    );
                    self.stack.push(element_1.xor(element_2).map_err(|error| {
                        Error::Element(expression_element.token.location, error)
                    })?);
                }
                OperatorExpressionObject::Operator(OperatorExpressionOperator::And) => {
                    let (element_2, element_1) = (
                        self.stack.pop().expect("Option state bug"),
                        self.stack.pop().expect("Option state bug"),
                    );
                    self.stack.push(element_1.and(element_2).map_err(|error| {
                        Error::Element(expression_element.token.location, error)
                    })?);
                }
                OperatorExpressionObject::Operator(OperatorExpressionOperator::Equal) => {
                    let (element_2, element_1) = (
                        self.stack.pop().expect("Option state bug"),
                        self.stack.pop().expect("Option state bug"),
                    );
                    self.stack.push(element_1.equal(element_2).map_err(|error| {
                        Error::Element(expression_element.token.location, error)
                    })?);
                }
                OperatorExpressionObject::Operator(OperatorExpressionOperator::NotEqual) => {
                    let (element_2, element_1) = (
                        self.stack.pop().expect("Option state bug"),
                        self.stack.pop().expect("Option state bug"),
                    );
                    self.stack
                        .push(element_1.not_equal(element_2).map_err(|error| {
                            Error::Element(expression_element.token.location, error)
                        })?);
                }
                OperatorExpressionObject::Operator(OperatorExpressionOperator::GreaterEqual) => {
                    let (element_2, element_1) = (
                        self.stack.pop().expect("Option state bug"),
                        self.stack.pop().expect("Option state bug"),
                    );
                    self.stack
                        .push(element_1.greater_equal(element_2).map_err(|error| {
                            Error::Element(expression_element.token.location, error)
                        })?);
                }
                OperatorExpressionObject::Operator(OperatorExpressionOperator::LesserEqual) => {
                    let (element_2, element_1) = (
                        self.stack.pop().expect("Option state bug"),
                        self.stack.pop().expect("Option state bug"),
                    );
                    self.stack
                        .push(element_1.lesser_equal(element_2).map_err(|error| {
                            Error::Element(expression_element.token.location, error)
                        })?);
                }
                OperatorExpressionObject::Operator(OperatorExpressionOperator::Greater) => {
                    let (element_2, element_1) = (
                        self.stack.pop().expect("Option state bug"),
                        self.stack.pop().expect("Option state bug"),
                    );
                    self.stack
                        .push(element_1.greater(element_2).map_err(|error| {
                            Error::Element(expression_element.token.location, error)
                        })?);
                }
                OperatorExpressionObject::Operator(OperatorExpressionOperator::Lesser) => {
                    let (element_2, element_1) = (
                        self.stack.pop().expect("Option state bug"),
                        self.stack.pop().expect("Option state bug"),
                    );
                    self.stack
                        .push(element_1.lesser(element_2).map_err(|error| {
                            Error::Element(expression_element.token.location, error)
                        })?);
                }
                OperatorExpressionObject::Operator(OperatorExpressionOperator::Addition) => {
                    let (element_2, element_1) = (
                        self.stack.pop().expect("Option state bug"),
                        self.stack.pop().expect("Option state bug"),
                    );
                    self.stack.push(element_1.add(element_2).map_err(|error| {
                        Error::Element(expression_element.token.location, error)
                    })?);
                }
                OperatorExpressionObject::Operator(OperatorExpressionOperator::Subtraction) => {
                    let (element_2, element_1) = (
                        self.stack.pop().expect("Option state bug"),
                        self.stack.pop().expect("Option state bug"),
                    );
                    self.stack
                        .push(element_1.subtract(element_2).map_err(|error| {
                            Error::Element(expression_element.token.location, error)
                        })?);
                }
                OperatorExpressionObject::Operator(OperatorExpressionOperator::Multiplication) => {
                    let (element_2, element_1) = (
                        self.stack.pop().expect("Option state bug"),
                        self.stack.pop().expect("Option state bug"),
                    );
                    self.stack
                        .push(element_1.multiply(element_2).map_err(|error| {
                            Error::Element(expression_element.token.location, error)
                        })?);
                }
                OperatorExpressionObject::Operator(OperatorExpressionOperator::Division) => {
                    let (element_2, element_1) = (
                        self.stack.pop().expect("Option state bug"),
                        self.stack.pop().expect("Option state bug"),
                    );
                    self.stack
                        .push(element_1.divide(element_2).map_err(|error| {
                            Error::Element(expression_element.token.location, error)
                        })?);
                }
                OperatorExpressionObject::Operator(OperatorExpressionOperator::Remainder) => {
                    let (element_2, element_1) = (
                        self.stack.pop().expect("Option state bug"),
                        self.stack.pop().expect("Option state bug"),
                    );
                    self.stack
                        .push(element_1.modulo(element_2).map_err(|error| {
                            Error::Element(expression_element.token.location, error)
                        })?);
                }
                OperatorExpressionObject::Operator(OperatorExpressionOperator::Casting) => {
                    let (element_2, element_1) = (
                        self.stack.pop().expect("Option state bug"),
                        self.stack.pop().expect("Option state bug"),
                    );
                    self.stack.push(element_1.cast(element_2).map_err(|error| {
                        Error::Element(expression_element.token.location, error)
                    })?);
                }
                OperatorExpressionObject::Operator(OperatorExpressionOperator::Negation) => {
                    let element = self.stack.pop().expect("Option state bug");
                    self.stack.push(element.negate().map_err(|error| {
                        Error::Element(expression_element.token.location, error)
                    })?);
                }
                OperatorExpressionObject::Operator(OperatorExpressionOperator::Not) => {
                    let element = self.stack.pop().expect("Option state bug");
                    self.stack.push(element.not().map_err(|error| {
                        Error::Element(expression_element.token.location, error)
                    })?);
                }
            }
        }

        match self.stack.pop() {
            Some(Element::Value(value)) => Ok(value),
            Some(Element::Place(place)) => Ok(place.value),
            _ => panic!("Type expressions cannot be evaluated (yet)"),
        }
    }

    pub fn evaluate_block(&mut self, block: BlockExpression) -> Result<Value, Error> {
        log::trace!("Block expression       : {}", block);

        let mut executor = Executor::new(Scope::new(Some(self.scope.clone())));
        for statement in block.statements {
            executor.execute(statement)?;
        }
        if let Some(expression) = block.expression {
            executor.evaluate(*expression)
        } else {
            Ok(Value::Void)
        }
    }
}
