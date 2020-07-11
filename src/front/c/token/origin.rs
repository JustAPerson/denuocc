// Licensed   under  the   Apache  License,   Version  2.0   <LICENSE-APACHE  or
// http://www.apache.org/licenses/LICENSE-2.0> or  the MIT  license <LICENSE-MIT
// or http://opensource.org/licenses/MIT>, at your option.  This file may not be
// copied, modified, or distributed except according to those terms.

//! Tracking where a token originates from

use std::collections::HashMap;
use std::rc::Rc;

use crate::front::c::preprocessor::MacroDef;
use crate::front::c::token::{PPToken, TextSpan};
use crate::front::c::tuctx::TUCtx;

/// Where a token was expanded from a macro
#[derive(Clone, Debug)]
pub struct MacroInvocation {
    pub definition: Rc<MacroDef>,
    pub name: PPToken,
    pub arguments: HashMap<String, Vec<PPToken>>,
}

#[derive(Clone, Debug)]
pub struct MacroResult {
    invocation: u32,
    in_index: u16,
    out_index: u16,
}

pub enum MacroTokenOrigin<'a> {
    Body(&'a PPToken),
    Argument(&'a PPToken),
}

impl MacroResult {
    pub fn new_param(invocation: u32, in_index: u16) -> MacroResult {
        assert!(in_index < 0x8000, "Macro arguments too long");
        MacroResult {
            invocation,
            in_index,
            out_index: 0xffff,
        }
    }

    pub fn new_body(invocation: u32, body_index: u16) -> MacroResult {
        assert!(body_index < 0x8000, "Macro body too long");
        let in_index = 0x8000 + body_index;
        MacroResult {
            invocation,
            in_index,
            out_index: 0xffff,
        }
    }

    pub fn update_out_index(&mut self, out_index: u16) {
        debug_assert!(out_index < 0xffff);
        self.out_index = out_index;
    }

    pub fn textspan(&self) -> &TextSpan {
        todo!()
    }

    pub fn is_arg(&self) -> bool {
        self.in_index < 0x8000
    }

    pub fn arg_index(&self) -> Option<usize> {
        if self.is_arg() {
            Some(self.in_index as usize)
        } else {
            None
        }
    }

    pub fn is_body(&self) -> bool {
        self.in_index >= 0x8000
    }

    pub fn body_index(&self) -> Option<usize> {
        if self.is_body() {
            Some((self.in_index - 0x8000) as usize)
        } else {
            None
        }
    }

    pub fn invocation_id(&self) -> u32 {
        self.invocation
    }

    pub fn invocation<'a>(&self, tuctx: &'a TUCtx) -> &'a MacroInvocation {
        &tuctx.macro_invocations[self.invocation as usize]
    }

    pub fn origin<'a>(&self, tuctx: &'a TUCtx) -> MacroTokenOrigin<'a> {
        let invocation = self.invocation(tuctx);
        if let Some(mut param_index) = self.arg_index() {
            debug_assert!(matches!(*invocation.definition, MacroDef::Function(..)));
            let function = invocation.definition.as_function();
            for param_name in &function.params {
                let param = &invocation.arguments[param_name];
                if param.len() >= param_index {
                    param_index -= param.len();
                } else {
                    return MacroTokenOrigin::Argument(&param[param_index]);
                }
            }
            unreachable!();
        } else if let Some(body_index) = self.body_index() {
            return MacroTokenOrigin::Body(
                &invocation.definition.replacement()[body_index as usize],
            );
        } else {
            unreachable!();
        }
    }

    pub fn input_token<'a>(&self, tuctx: &'a TUCtx) -> &'a PPToken {
        let invocation = self.invocation(tuctx);
        match self.origin(tuctx) {
            MacroTokenOrigin::Argument(token) => token,
            MacroTokenOrigin::Body(..) => &invocation.name,
        }
    }

    pub fn root_textspan<'a>(&self, tuctx: &'a TUCtx) -> &'a TextSpan {
        self.input_token(tuctx).origin.macro_root_textspan(tuctx)
    }
}

/// Where a token originates from
#[derive(Clone, Debug)]
pub enum TokenOrigin {
    Source(TextSpan),
    Macro(MacroResult),
}

impl TokenOrigin {
    // could potentially copy a similar method to PPToken
    pub fn macro_root_textspan<'a>(&'a self, tuctx: &'a TUCtx) -> &'a TextSpan {
        match &self {
            TokenOrigin::Source(span) => span,
            TokenOrigin::Macro(mresult) => mresult.root_textspan(tuctx),
        }
    }

    pub fn as_source_span(&self) -> TextSpan {
        match self {
            TokenOrigin::Source(span) => *span,
            _ => panic!(""),
        }
    }
}

impl std::convert::From<TextSpan> for TokenOrigin {
    fn from(span: TextSpan) -> TokenOrigin {
        TokenOrigin::Source(span)
    }
}
