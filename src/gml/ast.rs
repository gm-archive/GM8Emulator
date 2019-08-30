use super::lexer::Lexer;
use super::token::{Keyword, Operator, Separator, Token};

use std::error;
use std::fmt;

use std::iter::Peekable;

#[derive(Debug, PartialEq)]
pub struct AST<'a> {
    pub expressions: Vec<Expr<'a>>,
}

#[derive(Debug, PartialEq)]
pub enum Expr<'a> {
    LiteralIdentifier(&'a str),
    LiteralReal(f64),
    LiteralString(&'a str),

    Unary(Box<UnaryExpr<'a>>),
    Binary(Box<BinaryExpr<'a>>),

    DoUntil(Box<DoUntilExpr<'a>>),
    For(Box<ForExpr<'a>>),
    Function(Box<FunctionExpr<'a>>),
    Group(Vec<Expr<'a>>),
    If(Box<IfExpr<'a>>),
    Repeat(Box<RepeatExpr<'a>>),
    Switch(Box<SwitchExpr<'a>>),
    Var(Box<VarExpr<'a>>),
    With(Box<WithExpr<'a>>),
    While(Box<WhileExpr<'a>>),

    Case(Box<Expr<'a>>),
    Default,

    Continue,
    Break,
    Exit,
    Return(Box<Expr<'a>>),

    Nop,
}

#[derive(Debug, PartialEq)]
pub struct UnaryExpr<'a> {
    pub op: Operator,
    pub child: Expr<'a>,
}

#[derive(Debug, PartialEq)]
pub struct BinaryExpr<'a> {
    pub op: Operator,
    pub left: Expr<'a>,
    pub right: Expr<'a>,
}

#[derive(Debug, PartialEq)]
pub struct FunctionExpr<'a> {
    pub name: &'a str,
    pub params: Vec<Expr<'a>>,
}

#[derive(Debug, PartialEq)]
pub struct DoUntilExpr<'a> {
    pub cond: Expr<'a>,
    pub body: Expr<'a>,
}

#[derive(Debug, PartialEq)]
pub struct ForExpr<'a> {
    pub start: Expr<'a>,
    pub cond: Expr<'a>,
    pub step: Expr<'a>,

    pub body: Expr<'a>,
}

#[derive(Debug, PartialEq)]
pub struct IfExpr<'a> {
    pub cond: Expr<'a>,
    pub body: Expr<'a>,
    pub else_body: Option<Expr<'a>>,
}

#[derive(Debug, PartialEq)]
pub struct RepeatExpr<'a> {
    pub count: Expr<'a>,
    pub body: Expr<'a>,
}

#[derive(Debug, PartialEq)]
pub struct SwitchExpr<'a> {
    pub input: Expr<'a>,
    pub body: Expr<'a>,
}

#[derive(Debug, PartialEq)]
pub struct VarExpr<'a> {
    pub vars: Vec<&'a str>,
}

#[derive(Debug, PartialEq)]
pub struct WithExpr<'a> {
    pub target: Expr<'a>,
    pub body: Expr<'a>,
}

#[derive(Debug, PartialEq)]
pub struct WhileExpr<'a> {
    pub cond: Expr<'a>,
    pub body: Expr<'a>,
}

#[derive(Debug)]
pub struct Error {
    message: String,
}

impl Error {
    pub fn new(message: String) -> Self {
        Error { message }
    }
}

impl<'a> fmt::Display for Expr<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Expr::LiteralIdentifier(id) => write!(f, "{}", id),
            Expr::LiteralReal(r) => write!(f, "{}", r),
            Expr::LiteralString(s) => write!(f, "\"{}\"", s),

            Expr::Unary(unary) => write!(f, "({} {})", unary.op, unary.child),
            Expr::Binary(binary) => write!(f, "({} {} {})", binary.op, binary.left, binary.right),

            Expr::DoUntil(dountil) => write!(f, "(do {} until {})", dountil.body, dountil.cond),
            Expr::For(for_ex) => write!(
                f,
                "(for ({}, {}, {}) {})",
                for_ex.start, for_ex.cond, for_ex.step, for_ex.body
            ),
            Expr::Function(call) => write!(
                f,
                "(@{} {})",
                call.name,
                call.params
                    .iter()
                    .filter(|ex| **ex != Expr::Nop)
                    .fold(String::new(), |acc, fnname| acc + &format!("{} ", fnname))
                    .trim_end()
            ),
            Expr::Group(group) => write!(
                f,
                "<{}>",
                group
                    .iter()
                    .filter(|ex| **ex != Expr::Nop)
                    .fold(String::new(), |acc, expr| acc + &format!("{}, ", expr))
                    .trim_end_matches(|ch| ch == ' ' || ch == ',')
            ),
            Expr::If(if_ex) => match if_ex.else_body {
                Some(ref els) => write!(f, "(if {} {} {})", if_ex.cond, if_ex.body, els),
                None => write!(f, "(if {} {})", if_ex.cond, if_ex.body),
            },
            Expr::Repeat(repeat) => write!(f, "(repeat {} {})", repeat.count, repeat.body),
            Expr::Switch(switch) => write!(f, "(switch {} {})", switch.input, switch.body),
            Expr::Var(var) => write!(
                f,
                "(var {})",
                var.vars
                    .iter()
                    .fold(String::new(), |acc, varname| acc + &format!("{} ", varname))
                    .trim_end()
            ),
            Expr::With(with) => write!(f, "(with {} {})", with.target, with.body),
            Expr::While(while_ex) => write!(f, "(while {} {})", while_ex.cond, while_ex.body),

            Expr::Case(e) => write!(f, "(case {})", e),
            Expr::Default => write!(f, "(default)"),

            Expr::Continue => write!(f, "(continue)"),
            Expr::Break => write!(f, "(break)"),
            Expr::Exit => write!(f, "(exit)"),
            Expr::Return(e) => write!(f, "(return {})", e),

            Expr::Nop => write!(f, ""),
        }
    }
}

impl error::Error for Error {}
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

// TODO? This is not the prettiest.
macro_rules! expect_token {
    ( $token: expr, $line: expr, $($content: tt)* ) => ({
        match $token {
            Some(Token::$($content)*) => {},
            Some(t) => {
                return Err(Error::new(format!(
                    "Unexpected token {:?} on line {}; `{}` expected",
                    t, $line, Token::$($content)*,
                )));
            }
            None => {
                return Err(Error::new(format!(
                    "Unexpected EOF on line {}; `{}` expected",
                    $line, Token::$($content)*,
                )));
            }
        }
    });
}

impl<'a> AST<'a> {
    // TODO: make this const fn when Vec::new() const fn is stabilized
    pub fn empty() -> Self {
        AST {
            expressions: Vec::new(),
        }
    }

    pub fn new(source: &'a str) -> Result<Self, Error> {
        let mut lex = Lexer::new(source).peekable();
        let mut expressions = Vec::new();
        let mut line: usize = 1;

        loop {
            // Get the first token from the iterator, or exit the loop if there are no more
            match AST::read_line(&mut lex, &mut line) {
                Ok(Some(expr)) => {
                    // Filter top-level NOPs
                    if expr != Expr::Nop {
                        expressions.push(expr);
                    }
                }
                Ok(None) => break,
                Err(e) => return Err(e),
            }
        }

        Ok(AST { expressions })
    }

    fn read_line(lex: &mut Peekable<Lexer<'a>>, line: &mut usize) -> Result<Option<Expr<'a>>, Error> {
        let token = match lex.next() {
            Some(t) => t,
            None => return Ok(None), // EOF
        };

        // Use token type to determine what logic we should apply here
        match token {
            Token::Keyword(key) => {
                match key {
                    Keyword::Var => {
                        // Read var identifiers
                        if let Some(&Token::Identifier(id)) = lex.peek() {
                            lex.next();
                            let mut vars = Vec::with_capacity(1);
                            vars.push(id);

                            loop {
                                // Check if next token is a comma, if so, we expect another var name afterwards
                                if let Some(Token::Separator(Separator::Comma)) = lex.peek() {
                                    lex.next();
                                } else {
                                    break;
                                }

                                // Read one identifier and store it as a var name
                                if let Some(Token::Identifier(id)) = lex.peek() {
                                    vars.push(id);
                                    lex.next();
                                } else {
                                    break;
                                }
                            }

                            Ok(Some(Expr::Var(Box::new(VarExpr { vars }))))
                        } else {
                            // This doesn't do anything in GML. We could probably make it a NOP.
                            Ok(Some(Expr::Var(Box::new(VarExpr { vars: vec![] }))))
                        }
                    }

                    Keyword::Do => {
                        let body = AST::read_line(lex, line)?
                            .ok_or_else(|| Error::new(format!("Unexpected EOF after 'do' keyword (line {})", line)))?;
                        expect_token!(lex.next(), line, Keyword(Keyword::Until));
                        let (cond, op) = AST::read_binary_tree(lex, line, None, false, 0)?;
                        if op.is_some() {
                            unreachable!("read_binary_tree returned an operator");
                        }
                        Ok(Some(Expr::DoUntil(Box::new(DoUntilExpr { cond, body }))))
                    }

                    Keyword::If => {
                        let (cond, op) = AST::read_binary_tree(lex, line, None, false, 0)?;
                        if op.is_some() {
                            unreachable!("read_binary_tree returned an operator");
                        }
                        if lex.peek() == Some(&Token::Separator(Separator::Then)) {
                            lex.next();
                        }
                        let body = AST::read_line(lex, line)?
                            .ok_or_else(|| Error::new(format!("Unexpected EOF after 'do' keyword (line {})", line)))?;
                        let else_body = if lex.peek() == Some(&Token::Keyword(Keyword::Else)) {
                            lex.next(); // consume 'else'
                            Some(AST::read_line(lex, line)?.ok_or_else(|| {
                                Error::new(format!("Unexpected EOF after 'do' keyword (line {})", line))
                            })?)
                        } else {
                            None
                        };
                        Ok(Some(Expr::If(Box::new(IfExpr { cond, body, else_body }))))
                    }

                    Keyword::For => {
                        expect_token!(lex.next(), line, Separator(Separator::ParenLeft));
                        let start = AST::read_line(lex, line)?
                            .ok_or_else(|| Error::new(format!("Unexpected EOF during 'for' params (line {})", line)))?;
                        if lex.peek() == Some(&Token::Separator(Separator::Semicolon)) {
                            lex.next();
                        }
                        let (cond, op) = AST::read_binary_tree(lex, line, None, false, 0)?;
                        if op.is_some() {
                            unreachable!("read_binary_tree returned an operator");
                        }
                        if lex.peek() == Some(&Token::Separator(Separator::Semicolon)) {
                            lex.next();
                        }
                        let step = AST::read_line(lex, line)?
                            .ok_or_else(|| Error::new(format!("Unexpected EOF during 'for' params (line {})", line)))?;
                        while lex.peek() == Some(&Token::Separator(Separator::Semicolon)) {
                            lex.next();
                        }
                        expect_token!(lex.next(), line, Separator(Separator::ParenRight));
                        let body = AST::read_line(lex, line)?
                            .ok_or_else(|| Error::new(format!("Unexpected EOF after 'for' params (line {})", line)))?;
                        Ok(Some(Expr::For(Box::new(ForExpr {
                            start,
                            cond,
                            step,
                            body,
                        }))))
                    }

                    Keyword::Repeat => {
                        let (count, op) = AST::read_binary_tree(lex, line, None, false, 0)?;
                        if op.is_some() {
                            unreachable!("read_binary_tree returned an operator");
                        }
                        let body = AST::read_line(lex, line)?.ok_or_else(|| {
                            Error::new(format!("Unexpected EOF after 'repeat' condition (line {})", line))
                        })?;
                        Ok(Some(Expr::Repeat(Box::new(RepeatExpr { count, body }))))
                    }

                    Keyword::Switch => {
                        let (input, op) = AST::read_binary_tree(lex, line, None, false, 0)?;
                        if op.is_some() {
                            unreachable!("read_binary_tree returned an operator");
                        }
                        let body = AST::read_line(lex, line)?.ok_or_else(|| {
                            Error::new(format!("Unexpected EOF after 'repeat' condition (line {})", line))
                        })?;
                        Ok(Some(Expr::Switch(Box::new(SwitchExpr { input, body }))))
                    }

                    Keyword::With => {
                        let (target, op) = AST::read_binary_tree(lex, line, None, false, 0)?;
                        if op.is_some() {
                            unreachable!("read_binary_tree returned an operator");
                        }
                        let body = AST::read_line(lex, line)?.ok_or_else(|| {
                            Error::new(format!("Unexpected EOF after 'with' condition (line {})", line))
                        })?;
                        Ok(Some(Expr::With(Box::new(WithExpr { target, body }))))
                    }

                    Keyword::While => {
                        let (cond, op) = AST::read_binary_tree(lex, line, None, false, 0)?;
                        if op.is_some() {
                            unreachable!("read_binary_tree returned an operator");
                        }
                        let body = AST::read_line(lex, line)?.ok_or_else(|| {
                            Error::new(format!("Unexpected EOF after 'with' condition (line {})", line))
                        })?;
                        Ok(Some(Expr::While(Box::new(WhileExpr { cond, body }))))
                    }

                    Keyword::Case => {
                        let (expr, op) = AST::read_binary_tree(lex, line, None, false, 0)?;
                        if op.is_some() {
                            unreachable!("read_binary_tree returned an operator");
                        }
                        expect_token!(lex.next(), line, Separator(Separator::Colon));
                        Ok(Some(Expr::Case(Box::new(expr))))
                    }

                    Keyword::Default => {
                        expect_token!(lex.next(), line, Separator(Separator::Colon));
                        Ok(Some(Expr::Default))
                    }

                    Keyword::Break => Ok(Some(Expr::Break)),

                    Keyword::Continue => Ok(Some(Expr::Continue)),

                    Keyword::Exit => Ok(Some(Expr::Exit)),

                    Keyword::Return => {
                        let (val, op) = AST::read_binary_tree(lex, line, None, false, 0)?;
                        if op.is_some() {
                            unreachable!("read_binary_tree returned an operator");
                        }
                        Ok(Some(Expr::Return(Box::new(val))))
                    }

                    _ => {
                        return Err(Error::new(format!(
                            "Invalid Keyword at beginning of expression on line {}: {:?}",
                            line, key
                        )));
                    }
                }
            }

            Token::Identifier(id) => {
                // An expression starting with an identifier may be either an assignment or script/function.
                // This is determined by what type of token immediately follows it.
                let next_token = match lex.peek() {
                    Some(t) => t,
                    None => return Err(Error::new(format!("Stray identifier at EOF: {:?}", id))),
                };
                match next_token {
                    Token::Separator(ref sep) if *sep == Separator::ParenLeft => {
                        Ok(Some(AST::read_function_call(lex, line, id)?))
                    }
                    _ => {
                        let binary_tree = AST::read_binary_tree(lex, line, Some(token), true, 0)?;
                        if let Some(op) = binary_tree.1 {
                            Err(Error::new(format!(
                                "Stray operator {:?} in expression on line {}",
                                op, line,
                            )))
                        } else {
                            Ok(Some(binary_tree.0))
                        }
                    }
                }
            }

            Token::Separator(sep) => {
                match sep {
                    // Code contained in {} is treated here as one single expression, called a Group.
                    Separator::BraceLeft => {
                        let mut inner_expressions = Vec::new();
                        loop {
                            match lex.peek() {
                                Some(Token::Separator(Separator::BraceRight)) => {
                                    lex.next();
                                    break Ok(Some(Expr::Group(inner_expressions)));
                                }
                                _ => match AST::read_line(lex, line) {
                                    Ok(Some(Expr::Nop)) => continue,
                                    Ok(Some(e)) => inner_expressions.push(e),
                                    Ok(None) => break Err(Error::new("Unclosed brace at EOF".to_string())),
                                    Err(e) => break Err(e),
                                },
                            }
                        }
                    }

                    // An assignment may start with an open-parenthesis, eg: (1).x = 400;
                    Separator::ParenLeft => {
                        let binary_tree =
                            AST::read_binary_tree(lex, line, Some(Token::Separator(Separator::ParenLeft)), true, 0)?;
                        if let Some(op) = binary_tree.1 {
                            Err(Error::new(format!(
                                "Stray operator {:?} in expression on line {}",
                                op, line,
                            )))
                        } else {
                            Ok(Some(binary_tree.0))
                        }
                    }

                    // A semicolon is treated as a line of code which does nothing.
                    Separator::Semicolon => Ok(Some(Expr::Nop)),

                    // Default
                    _ => {
                        return Err(Error::new(format!(
                            "Invalid Separator at beginning of expression on line {}: {:?}",
                            line, sep
                        )));
                    }
                }
            }

            _ => {
                return Err(Error::new(format!(
                    "Invalid token at beginning of expression on line {}: {:?}",
                    line, token
                )));
            }
        }
    }

    fn read_binary_tree(
        lex: &mut Peekable<Lexer<'a>>,
        line: &mut usize,
        first_token: Option<Token<'a>>, // Sometimes we've already parsed the first token, so it should be put here.
        expect_assignment: bool,        // Do we expect the first op to be an assignment?
        lowest_prec: u8,                // We are not allowed to go below this operator precedence in this tree.
                                        // If we do, we'll return the next op.
    ) -> Result<(Expr<'a>, Option<Operator>), Error> {
        // Get the first expression before any operators
        let mut lhs = AST::read_btree_expression(lex, line, first_token)?;

        // Check if the next token is an operator
        let next_token = lex.peek();
        match next_token {
            Some(&Token::Operator(_)) => {
                if let Some(Token::Operator(mut op)) = lex.next() {
                    // '=' can be either an assignment or equality check (==) in GML.
                    // So if we're not expecting an assignment operator, it should be seen as a comparator instead.
                    if (op == Operator::Assign) && (!expect_assignment) {
                        op = Operator::Equal;
                    }

                    // Now, loop until there are no more buffered operators.
                    loop {
                        // Here, we get the precedence of the operator we found.
                        // If this returns None, it's probably an assignment,
                        // so we use that in conjunction with an if-let to check its validity.
                        if let Some(precedence) = AST::get_op_precedence(&op) {
                            // this op is invalid if an assignment is expected
                            if expect_assignment {
                                break Err(Error::new(format!(
                                    "Invalid operator {:?} found, expected assignment (line {})",
                                    op, line,
                                )));
                            } else {
                                // If this op has lower prec than we're allowed to read, we have to return it here.
                                if precedence < lowest_prec {
                                    break Ok((lhs, Some(op)));
                                } else {
                                    // We're allowed to use the next operator. Let's read an RHS to put on after it.
                                    // You might be thinking "precedence + 1" is counter-intuitive -
                                    // "precedence" would make more sense, right?
                                    // Well, the difference is left-to-right vs right-to-left construction.
                                    // This way, 1/2/3 is correctly built as (1/2)/3 rather than 1/(2/3).
                                    let rhs = AST::read_binary_tree(lex, line, None, false, precedence + 1)?;
                                    if let Some(next_op) = rhs.1 {
                                        // There's another operator even after the RHS.
                                        if let Some(next_prec) = AST::get_op_precedence(&next_op) {
                                            if next_prec < lowest_prec {
                                                // This next op is lower than we're allowed to go, so we must return it
                                                break Ok((
                                                    Expr::Binary(Box::new(BinaryExpr {
                                                        op: op,
                                                        left: lhs,
                                                        right: rhs.0,
                                                    })),
                                                    Some(next_op),
                                                ));
                                            } else {
                                                // Update LHS by sticking RHS onto it,
                                                // set op to the new operator, and go round again.
                                                lhs = Expr::Binary(Box::new(BinaryExpr {
                                                    op: op,
                                                    left: lhs,
                                                    right: rhs.0,
                                                }));
                                                op = next_op;
                                            }
                                        } else {
                                            // Precedence would already have been checked by the returning function.
                                            unreachable!()
                                        }
                                    } else {
                                        // No more operators so let's put our lhs and rhs together.
                                        break Ok((
                                            Expr::Binary(Box::new(BinaryExpr {
                                                op: op,
                                                left: lhs,
                                                right: rhs.0,
                                            })),
                                            None,
                                        ));
                                    }
                                }
                            }
                        } else {
                            // this op is invalid if assignment not expected, OR if it's a unary operator
                            // (those have no precedence so they pass the previous test.)
                            if !expect_assignment || op == Operator::Not || op == Operator::Complement {
                                break Err(Error::new(format!(
                                    "Invalid operator {:?} found, expected evaluable (line {})",
                                    op, line,
                                )));
                            } else {
                                // No need to do precedence on an assignment, so just grab RHS and return
                                let rhs = AST::read_binary_tree(lex, line, None, false, lowest_prec)?;
                                if let Some(op) = rhs.1 {
                                    break Err(Error::new(format!(
                                        "Stray operator {:?} in expression on line {}",
                                        op, line,
                                    )));
                                } else {
                                    break Ok((
                                        Expr::Binary(Box::new(BinaryExpr {
                                            op: op,
                                            left: lhs,
                                            right: rhs.0,
                                        })),
                                        None,
                                    ));
                                }
                            }
                        }
                    }
                } else {
                    unreachable!()
                }
            }
            _ => {
                if expect_assignment {
                    Err(Error::new(format!(
                        "Invalid token {:?} when expecting assignment operator on line {}",
                        next_token, line,
                    )))
                } else {
                    Ok((lhs, None))
                }
            }
        }
    }

    fn read_btree_expression(
        lex: &mut Peekable<Lexer<'a>>,
        line: &mut usize,
        first_token: Option<Token<'a>>,
    ) -> Result<Expr<'a>, Error> {
        // Get first token and match it
        let mut lhs = match if first_token.is_some() { first_token } else { lex.next() } {
            Some(Token::Separator(ref sep)) if *sep == Separator::ParenLeft => {
                let binary_tree = AST::read_binary_tree(lex, line, None, false, 0)?;
                if lex.next() != Some(Token::Separator(Separator::ParenRight)) {
                    return Err(Error::new(format!(
                        "Unclosed parenthesis in binary tree on line {}",
                        line
                    )));
                } else if let Some(op) = binary_tree.1 {
                    return Err(Error::new(format!(
                        "Stray operator {:?} in expression on line {}",
                        op, line,
                    )));
                }
                binary_tree.0
            }
            Some(Token::Operator(op)) => {
                if op == Operator::Add || op == Operator::Subtract || op == Operator::Not || op == Operator::Complement
                {
                    Expr::Unary(Box::new(UnaryExpr {
                        op: op,
                        child: AST::read_btree_expression(lex, line, None)?,
                    }))
                } else {
                    return Err(Error::new(format!(
                        "Invalid unary operator {:?} in expression on line {}",
                        op, line,
                    )));
                }
            }
            Some(Token::Identifier(t)) => {
                if lex.peek() == Some(&Token::Separator(Separator::ParenLeft)) {
                    AST::read_function_call(lex, line, t)?
                } else {
                    Expr::LiteralIdentifier(t)
                }
            }

            Some(Token::Real(t)) => Expr::LiteralReal(t),
            Some(Token::String(t)) => Expr::LiteralString(t),
            Some(t) => {
                return Err(Error::new(format!(
                    "Invalid token while scanning binary tree on line {}: {:?}",
                    line, t
                )));
            }
            None => {
                return Err(Error::new(format!(
                    "Found EOF unexpectedly while reading binary tree (line {})",
                    line
                )));
            }
        };

        // Do we need to amend this LHS at all?
        loop {
            match lex.peek() {
                Some(Token::Separator(ref sep)) if *sep == Separator::BracketLeft => {
                    lex.next();
                    let mut dimensions = Vec::new();
                    if lex.peek() == Some(&Token::Separator(Separator::BracketRight)) {
                        lex.next();
                    } else {
                        loop {
                            let (dim, op) = AST::read_binary_tree(lex, line, None, false, 0)?;
                            if op.is_some() {
                                unreachable!("read_binary_tree returned an operator");
                            }
                            dimensions.push(dim);
                            match lex.next() {
                                Some(Token::Separator(Separator::BracketRight)) => break,
                                Some(Token::Separator(Separator::Comma)) => {
                                    if lex.peek() == Some(&Token::Separator(Separator::BracketRight)) {
                                        lex.next();
                                        break;
                                    }
                                }
                                Some(t) => {
                                    return Err(Error::new(format!(
                                        "Invalid token {:?}, expected expression (line {})",
                                        t, line
                                    )));
                                }
                                None => {
                                    return Err(Error::new(format!(
                                        "Found EOF unexpectedly while reading array accessor (line {})",
                                        line
                                    )));
                                }
                            }
                        }
                    }
                    lhs = Expr::Binary(Box::new(BinaryExpr {
                        op: Operator::Index,
                        left: lhs,
                        right: Expr::Group(dimensions),
                    }));
                }

                Some(Token::Separator(ref sep)) if *sep == Separator::Period => {
                    lex.next();
                    lhs = match lex.next() {
                        Some(Token::Identifier(id)) => Expr::Binary(Box::new(BinaryExpr {
                            op: Operator::Deref,
                            left: lhs,
                            right: Expr::LiteralIdentifier(id),
                        })),
                        Some(t) => {
                            return Err(Error::new(format!(
                                "Unexpected token {:?} following deref on line {}",
                                t, line
                            )));
                        }
                        None => {
                            return Err(Error::new(format!(
                                "Found EOF unexpectedly while reading binary tree (line {})",
                                line
                            )));
                        }
                    }
                }
                _ => break,
            }
        }

        Ok(lhs)
    }

    fn read_function_call(
        lex: &mut Peekable<Lexer<'a>>,
        line: &mut usize,
        function_name: &'a str,
    ) -> Result<Expr<'a>, Error> {
        expect_token!(lex.next(), line, Separator(Separator::ParenLeft));

        let mut params = Vec::new();
        if lex.peek() == Some(&Token::Separator(Separator::ParenRight)) {
            lex.next();
        } else {
            loop {
                let (param, op) = AST::read_binary_tree(lex, line, None, false, 0)?;
                if op.is_some() {
                    unreachable!("read_binary_tree returned an operator");
                }
                params.push(param);
                match lex.next() {
                    Some(Token::Separator(Separator::ParenRight)) => break,
                    Some(Token::Separator(Separator::Comma)) => {
                        if lex.peek() == Some(&Token::Separator(Separator::ParenRight)) {
                            lex.next();
                            break;
                        }
                    }
                    Some(t) => {
                        return Err(Error::new(format!(
                            "Invalid token {:?}, expected expression (line {})",
                            t, line
                        )));
                    }
                    None => {
                        return Err(Error::new(format!(
                            "Found EOF unexpectedly while reading function call (line {})",
                            line
                        )));
                    }
                }
            }
        }
        Ok(Expr::Function(Box::new(FunctionExpr {
            name: function_name,
            params: params,
        })))
    }

    fn get_op_precedence(op: &Operator) -> Option<u8> {
        match op {
            Operator::Add => Some(4),
            Operator::Subtract => Some(4),
            Operator::Multiply => Some(5),
            Operator::Divide => Some(5),
            Operator::IntDivide => Some(5),
            Operator::BinaryAnd => Some(2),
            Operator::BinaryOr => Some(2),
            Operator::BinaryXor => Some(2),
            Operator::Assign => None,
            Operator::Not => None,
            Operator::LessThan => Some(1),
            Operator::GreaterThan => Some(1),
            Operator::AssignAdd => None,
            Operator::AssignSubtract => None,
            Operator::AssignMultiply => None,
            Operator::AssignDivide => None,
            Operator::AssignBinaryAnd => None,
            Operator::AssignBinaryOr => None,
            Operator::AssignBinaryXor => None,
            Operator::Equal => Some(1),
            Operator::NotEqual => Some(1),
            Operator::LessThanOrEqual => Some(1),
            Operator::GreaterThanOrEqual => Some(1),
            Operator::Modulo => Some(5),
            Operator::And => Some(0),
            Operator::Or => Some(0),
            Operator::Xor => Some(0),
            Operator::BinaryShiftLeft => Some(3),
            Operator::BinaryShiftRight => Some(3),
            Operator::Complement => None,
            Operator::Deref => None,
            Operator::Index => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_nothing() {
        // Empty string
        assert_ast("", Some(vec![]))
    }

    #[test]
    fn test_assignment_op_assign() {
        assert_ast(
            // Simple assignment - Assign
            "a = 1",
            Some(vec![Expr::Binary(Box::new(BinaryExpr {
                op: Operator::Assign,
                left: Expr::LiteralIdentifier("a"),
                right: Expr::LiteralReal(1.0),
            }))]),
        )
    }

    #[test]
    fn test_assignment_op_add() {
        assert_ast(
            // Simple assignment - AssignAdd
            "b += 2",
            Some(vec![Expr::Binary(Box::new(BinaryExpr {
                op: Operator::AssignAdd,
                left: Expr::LiteralIdentifier("b"),
                right: Expr::LiteralReal(2.0),
            }))]),
        )
    }

    #[test]
    fn test_assignment_op_subtract() {
        assert_ast(
            // Simple assignment - AssignSubtract
            "c -= 3",
            Some(vec![Expr::Binary(Box::new(BinaryExpr {
                op: Operator::AssignSubtract,
                left: Expr::LiteralIdentifier("c"),
                right: Expr::LiteralReal(3.0),
            }))]),
        )
    }

    #[test]
    fn test_assignment_op_multiply() {
        assert_ast(
            // Simple assignment - AssignMultiply
            "d *= 4",
            Some(vec![Expr::Binary(Box::new(BinaryExpr {
                op: Operator::AssignMultiply,
                left: Expr::LiteralIdentifier("d"),
                right: Expr::LiteralReal(4.0),
            }))]),
        )
    }

    #[test]
    fn test_assignment_op_divide() {
        assert_ast(
            // Simple assignment - AssignDivide
            "e /= 5",
            Some(vec![Expr::Binary(Box::new(BinaryExpr {
                op: Operator::AssignDivide,
                left: Expr::LiteralIdentifier("e"),
                right: Expr::LiteralReal(5.0),
            }))]),
        )
    }

    #[test]
    fn test_assignment_op_and() {
        assert_ast(
            // Simple assignment - AssignBinaryAnd
            "f &= 6",
            Some(vec![Expr::Binary(Box::new(BinaryExpr {
                op: Operator::AssignBinaryAnd,
                left: Expr::LiteralIdentifier("f"),
                right: Expr::LiteralReal(6.0),
            }))]),
        )
    }

    #[test]
    fn test_assignment_op_or() {
        assert_ast(
            // Simple assignment - AssignBinaryOr
            "g |= 7",
            Some(vec![Expr::Binary(Box::new(BinaryExpr {
                op: Operator::AssignBinaryOr,
                left: Expr::LiteralIdentifier("g"),
                right: Expr::LiteralReal(7.0),
            }))]),
        )
    }

    #[test]
    fn test_assignment_op_xor() {
        assert_ast(
            // Simple assignment - AssignBinaryXor
            "h ^= 8",
            Some(vec![Expr::Binary(Box::new(BinaryExpr {
                op: Operator::AssignBinaryXor,
                left: Expr::LiteralIdentifier("h"),
                right: Expr::LiteralReal(8.0),
            }))]),
        )
    }

    #[test]
    #[should_panic]
    fn test_assignment_op_invalid() {
        // Assignment syntax - Multiply - should fail
        // Note: chose "Multiply" specifically as it cannot be unary, unlike Add or Subtract
        assert_ast("i * 9", None);
    }

    #[test]
    #[should_panic]
    fn test_assignment_op_not() {
        // Assignment syntax - Not - should fail
        assert_ast("j ! 10", None);
    }

    #[test]
    #[should_panic]
    fn test_assignment_op_complement() {
        // Assignment syntax - Complement - should fail
        assert_ast("k ~ 11", None);
    }

    #[test]
    fn test_assignment_lhs() {
        assert_ast(
            // Assignment with deref and index on lhs
            "a.b[c] += d;",
            Some(vec![Expr::Binary(Box::new(BinaryExpr {
                op: Operator::AssignAdd,
                left: Expr::Binary(Box::new(BinaryExpr {
                    op: Operator::Index,
                    left: Expr::Binary(Box::new(BinaryExpr {
                        op: Operator::Deref,
                        left: Expr::LiteralIdentifier("a"),
                        right: Expr::LiteralIdentifier("b"),
                    })),
                    right: Expr::Group(vec![Expr::LiteralIdentifier("c")]),
                })),
                right: Expr::LiteralIdentifier("d"),
            }))]),
        );
    }

    #[test]
    fn test_assignment_2d_index() {
        assert_ast(
            // Arbitrary chains of deref, 1- and 2-dimension index ops on both lhs and rhs
            "a.b[c].d.e[f,g]=h[i,j].k",
            Some(vec![Expr::Binary(Box::new(BinaryExpr {
                op: Operator::Assign,
                left: Expr::Binary(Box::new(BinaryExpr {
                    op: Operator::Index,
                    left: Expr::Binary(Box::new(BinaryExpr {
                        op: Operator::Deref,
                        left: Expr::Binary(Box::new(BinaryExpr {
                            op: Operator::Deref,
                            left: Expr::Binary(Box::new(BinaryExpr {
                                op: Operator::Index,
                                left: Expr::Binary(Box::new(BinaryExpr {
                                    op: Operator::Deref,
                                    left: Expr::LiteralIdentifier("a"),
                                    right: Expr::LiteralIdentifier("b"),
                                })),
                                right: Expr::Group(vec![Expr::LiteralIdentifier("c")]),
                            })),
                            right: Expr::LiteralIdentifier("d"),
                        })),
                        right: Expr::LiteralIdentifier("e"),
                    })),
                    right: Expr::Group(vec![Expr::LiteralIdentifier("f"), Expr::LiteralIdentifier("g")]),
                })),
                right: Expr::Binary(Box::new(BinaryExpr {
                    op: Operator::Deref,
                    left: Expr::Binary(Box::new(BinaryExpr {
                        op: Operator::Index,
                        left: Expr::LiteralIdentifier("h"),
                        right: Expr::Group(vec![Expr::LiteralIdentifier("i"), Expr::LiteralIdentifier("j")]),
                    })),
                    right: Expr::LiteralIdentifier("k"),
                })),
            }))]),
        );
    }

    #[test]
    fn test_assignment_lhs_expression() {
        assert_ast(
            // Assignment whose LHS is an expression-deref
            "(a + 1).x = 400;",
            Some(vec![Expr::Binary(Box::new(BinaryExpr {
                op: Operator::Assign,
                left: Expr::Binary(Box::new(BinaryExpr {
                    op: Operator::Deref,
                    left: Expr::Binary(Box::new(BinaryExpr {
                        op: Operator::Add,
                        left: Expr::LiteralIdentifier("a"),
                        right: Expr::LiteralReal(1.0),
                    })),
                    right: Expr::LiteralIdentifier("x"),
                })),
                right: Expr::LiteralReal(400.0),
            }))]),
        );
    }

    #[test]
    fn test_assignment_assign_equal() {
        assert_ast(
            // Differentiation between usages of '=' - simple
            "a=b=c",
            Some(vec![Expr::Binary(Box::new(BinaryExpr {
                op: Operator::Assign,
                left: Expr::LiteralIdentifier("a"),
                right: Expr::Binary(Box::new(BinaryExpr {
                    op: Operator::Equal,
                    left: Expr::LiteralIdentifier("b"),
                    right: Expr::LiteralIdentifier("c"),
                })),
            }))]),
        );
    }

    #[test]
    fn test_assignment_assign_equal_complex() {
        assert_ast(
            // Differentiation between usages of '=' - complex
            "(a=b).c[d=e]=f[g=h]=i",
            Some(vec![Expr::Binary(Box::new(BinaryExpr {
                op: Operator::Assign,
                left: Expr::Binary(Box::new(BinaryExpr {
                    op: Operator::Index,
                    left: Expr::Binary(Box::new(BinaryExpr {
                        op: Operator::Deref,
                        left: Expr::Binary(Box::new(BinaryExpr {
                            op: Operator::Equal,
                            left: Expr::LiteralIdentifier("a"),
                            right: Expr::LiteralIdentifier("b"),
                        })),
                        right: Expr::LiteralIdentifier("c"),
                    })),
                    right: Expr::Group(vec![Expr::Binary(Box::new(BinaryExpr {
                        op: Operator::Equal,
                        left: Expr::LiteralIdentifier("d"),
                        right: Expr::LiteralIdentifier("e"),
                    }))]),
                })),
                right: Expr::Binary(Box::new(BinaryExpr {
                    op: Operator::Equal,
                    left: Expr::Binary(Box::new(BinaryExpr {
                        op: Operator::Index,
                        left: Expr::LiteralIdentifier("f"),
                        right: Expr::Group(vec![Expr::Binary(Box::new(BinaryExpr {
                            op: Operator::Equal,
                            left: Expr::LiteralIdentifier("g"),
                            right: Expr::LiteralIdentifier("h"),
                        }))]),
                    })),
                    right: Expr::LiteralIdentifier("i"),
                })),
            }))]),
        );
    }

    #[test]
    #[should_panic]
    fn test_deref_not_id() {
        // Invalid use of deref operator
        assert_ast("a..=1", None)
    }

    #[test]
    fn test_btree_unary_positive() {
        assert_ast(
            // Binary tree format - unary operator - positive
            "a=+1",
            Some(vec![Expr::Binary(Box::new(BinaryExpr {
                op: Operator::Assign,
                left: Expr::LiteralIdentifier("a"),
                right: Expr::Unary(Box::new(UnaryExpr {
                    op: Operator::Add,
                    child: Expr::LiteralReal(1.0),
                })),
            }))]),
        )
    }

    #[test]
    fn test_btree_unary_subtract() {
        assert_ast(
            // Binary tree format - unary operator - negative
            "a=-1",
            Some(vec![Expr::Binary(Box::new(BinaryExpr {
                op: Operator::Assign,
                left: Expr::LiteralIdentifier("a"),
                right: Expr::Unary(Box::new(UnaryExpr {
                    op: Operator::Subtract,
                    child: Expr::LiteralReal(1.0),
                })),
            }))]),
        )
    }

    #[test]
    fn test_btree_unary_complement() {
        assert_ast(
            // Binary tree format - unary operator - complement
            "a=~1",
            Some(vec![Expr::Binary(Box::new(BinaryExpr {
                op: Operator::Assign,
                left: Expr::LiteralIdentifier("a"),
                right: Expr::Unary(Box::new(UnaryExpr {
                    op: Operator::Complement,
                    child: Expr::LiteralReal(1.0),
                })),
            }))]),
        )
    }

    #[test]
    fn test_btree_unary_not() {
        assert_ast(
            // Binary tree format - unary operator - negative
            "a=!1",
            Some(vec![Expr::Binary(Box::new(BinaryExpr {
                op: Operator::Assign,
                left: Expr::LiteralIdentifier("a"),
                right: Expr::Unary(Box::new(UnaryExpr {
                    op: Operator::Not,
                    child: Expr::LiteralReal(1.0),
                })),
            }))]),
        )
    }

    #[test]
    fn test_btree_unary_syntax() {
        assert_ast(
            // Binary tree format - unary operators - syntax parse test
            "a = 1+!~-b.c[+d]-2--3", // (- (- (+ 1 2) 3) 4)
            Some(vec![Expr::Binary(Box::new(BinaryExpr {
                op: Operator::Assign,
                left: Expr::LiteralIdentifier("a"),
                right: Expr::Binary(Box::new(BinaryExpr {
                    op: Operator::Subtract,
                    left: Expr::Binary(Box::new(BinaryExpr {
                        op: Operator::Subtract,
                        left: Expr::Binary(Box::new(BinaryExpr {
                            op: Operator::Add,
                            left: Expr::LiteralReal(1.0),
                            right: Expr::Unary(Box::new(UnaryExpr {
                                op: Operator::Not,
                                child: Expr::Unary(Box::new(UnaryExpr {
                                    op: Operator::Complement,
                                    child: Expr::Unary(Box::new(UnaryExpr {
                                        op: Operator::Subtract,
                                        child: Expr::Binary(Box::new(BinaryExpr {
                                            op: Operator::Index,
                                            left: Expr::Binary(Box::new(BinaryExpr {
                                                op: Operator::Deref,
                                                left: Expr::LiteralIdentifier("b"),
                                                right: Expr::LiteralIdentifier("c"),
                                            })),
                                            right: Expr::Group(vec![Expr::Unary(Box::new(UnaryExpr {
                                                op: Operator::Add,
                                                child: Expr::LiteralIdentifier("d"),
                                            }))]),
                                        })),
                                    })),
                                })),
                            })),
                        })),
                        right: Expr::LiteralReal(2.0),
                    })),
                    right: Expr::Unary(Box::new(UnaryExpr {
                        op: Operator::Subtract,
                        child: Expr::LiteralReal(3.0),
                    })),
                })),
            }))]),
        )
    }

    #[test]
    fn test_btree_unary_grouping() {
        assert_ast(
            // Unary operator applied to sub-tree
            "a = ~(b + 1)",
            Some(vec![Expr::Binary(Box::new(BinaryExpr {
                op: Operator::Assign,
                left: Expr::LiteralIdentifier("a"),
                right: Expr::Unary(Box::new(UnaryExpr {
                    op: Operator::Complement,
                    child: Expr::Binary(Box::new(BinaryExpr {
                        op: Operator::Add,
                        left: Expr::LiteralIdentifier("b"),
                        right: Expr::LiteralReal(1.0),
                    })),
                })),
            }))]),
        )
    }

    #[test]
    fn test_function_syntax() {
        assert_ast(
            // Function call syntax
            "instance_create(random(800), random(608,), apple);",
            Some(vec![Expr::Function(Box::new(FunctionExpr {
                name: "instance_create",
                params: vec![
                    Expr::Function(Box::new(FunctionExpr {
                        name: "random",
                        params: vec![Expr::LiteralReal(800.0)],
                    })),
                    Expr::Function(Box::new(FunctionExpr {
                        name: "random",
                        params: vec![Expr::LiteralReal(608.0)],
                    })),
                    Expr::LiteralIdentifier("apple"),
                ],
            }))]),
        )
    }

    #[test]
    fn test_for_syntax_standard() {
        assert_ast(
            // For-loop syntax - standard
            "for(i = 0; i < 10; i += 1) { a = 1; b = c;}",
            Some(vec![Expr::For(Box::new(ForExpr {
                start: Expr::Binary(Box::new(BinaryExpr {
                    op: Operator::Assign,
                    left: Expr::LiteralIdentifier("i"),
                    right: Expr::LiteralReal(0.0),
                })),
                cond: Expr::Binary(Box::new(BinaryExpr {
                    op: Operator::LessThan,
                    left: Expr::LiteralIdentifier("i"),
                    right: Expr::LiteralReal(10.0),
                })),
                step: Expr::Binary(Box::new(BinaryExpr {
                    op: Operator::AssignAdd,
                    left: Expr::LiteralIdentifier("i"),
                    right: Expr::LiteralReal(1.0),
                })),
                body: Expr::Group(vec![
                    Expr::Binary(Box::new(BinaryExpr {
                        op: Operator::Assign,
                        left: Expr::LiteralIdentifier("a"),
                        right: Expr::LiteralReal(1.0),
                    })),
                    Expr::Binary(Box::new(BinaryExpr {
                        op: Operator::Assign,
                        left: Expr::LiteralIdentifier("b"),
                        right: Expr::LiteralIdentifier("c"),
                    })),
                ]),
            }))]),
        )
    }

    #[test]
    fn test_for_syntax_no_sep() {
        assert_ast(
            // For-loop syntax - no separators
            "for(i=0 i<10 i+=1) c=3",
            Some(vec![Expr::For(Box::new(ForExpr {
                start: Expr::Binary(Box::new(BinaryExpr {
                    op: Operator::Assign,
                    left: Expr::LiteralIdentifier("i"),
                    right: Expr::LiteralReal(0.0),
                })),
                cond: Expr::Binary(Box::new(BinaryExpr {
                    op: Operator::LessThan,
                    left: Expr::LiteralIdentifier("i"),
                    right: Expr::LiteralReal(10.0),
                })),
                step: Expr::Binary(Box::new(BinaryExpr {
                    op: Operator::AssignAdd,
                    left: Expr::LiteralIdentifier("i"),
                    right: Expr::LiteralReal(1.0),
                })),
                body: Expr::Binary(Box::new(BinaryExpr {
                    op: Operator::Assign,
                    left: Expr::LiteralIdentifier("c"),
                    right: Expr::LiteralReal(3.0),
                })),
            }))]),
        )
    }

    #[test]
    fn test_for_syntax_random_sep() {
        assert_ast(
            // For-loop syntax - arbitrary semicolons
            "for(i=0; i<10 i+=1; ;) {d=4}",
            Some(vec![Expr::For(Box::new(ForExpr {
                start: Expr::Binary(Box::new(BinaryExpr {
                    op: Operator::Assign,
                    left: Expr::LiteralIdentifier("i"),
                    right: Expr::LiteralReal(0.0),
                })),
                cond: Expr::Binary(Box::new(BinaryExpr {
                    op: Operator::LessThan,
                    left: Expr::LiteralIdentifier("i"),
                    right: Expr::LiteralReal(10.0),
                })),
                step: Expr::Binary(Box::new(BinaryExpr {
                    op: Operator::AssignAdd,
                    left: Expr::LiteralIdentifier("i"),
                    right: Expr::LiteralReal(1.0),
                })),
                body: Expr::Group(vec![Expr::Binary(Box::new(BinaryExpr {
                    op: Operator::Assign,
                    left: Expr::LiteralIdentifier("d"),
                    right: Expr::LiteralReal(4.0),
                }))]),
            }))]),
        )
    }

    #[test]
    fn test_var_syntax() {
        assert_ast(
            // var syntax - basic constructions
            "var a; var b, c",
            Some(vec![
                Expr::Var(Box::new(VarExpr { vars: vec!["a"] })),
                Expr::Var(Box::new(VarExpr { vars: vec!["b", "c"] })),
            ]),
        )
    }

    #[test]
    fn test_var_syntax_complex() {
        assert_ast(
            // var syntax - unusual valid constructions
            "var; var a,b,; var c,var",
            Some(vec![
                Expr::Var(Box::new(VarExpr { vars: vec![] })),
                Expr::Var(Box::new(VarExpr { vars: vec!["a", "b"] })),
                Expr::Var(Box::new(VarExpr { vars: vec!["c"] })),
                Expr::Var(Box::new(VarExpr { vars: vec![] })),
            ]),
        )
    }

    #[test]
    #[should_panic]
    fn test_var_invalid_comma() {
        // var syntax - invalid comma
        assert_ast("var, a;", None)
    }

    fn assert_ast(input: &str, expected_output: Option<Vec<Expr>>) {
        match AST::new(input) {
            Ok(ast) => {
                if let Some(e) = expected_output {
                    assert_eq!(ast.expressions, e);
                }
            }
            Err(e) => panic!("AST test encountered error: '{}' for input: {}", e, input),
        }
    }
}