/*
 * Copyright 2016 - 2020 Andreas Nordal
 *
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use ::situation::Transition;
use ::situation::WhatNow;
use ::situation::flush;
use ::situation::COLOR_KWD;
use ::situation::COLOR_MAGIC;
use ::situation::COLOR_HERE;
use ::situation::COLOR_VAR;

use ::microparsers::prefixlen;
use ::microparsers::predlen;
use ::microparsers::identifierlen;
use ::microparsers::is_whitespace;
use ::microparsers::is_word;

use ::commonstrcmd::CommonStrCmdResult;
use ::commonstrcmd::common_str_cmd;

use ::sitcase::SitIn;
use ::sitcmd::SitNormal;
use ::sitcmd::SitCmd;
use ::sitcomment::SitComment;
use ::sitextent::SitExtent;
use ::sitrvalue::SitRvalue;
use ::sitstrdq::SitStrDq;
use ::sitstrphantom::SitStrPhantom;
use ::sitstrsqesc::SitStrSqEsc;
use ::situntilbyte::SitUntilByte;
use ::sitvec::SitVec;

pub fn keyword_or_command(
	end_trigger :u16,
	horizon: &[u8],
	i: usize,
	is_horizon_lengthenable: bool,
) -> WhatNow {
	if horizon[i] == b'(' {
		return WhatNow{
			tri: Transition::Push(Box::new(SitNormal{
				end_trigger: u16::from(b')'), end_replace: None
			})), pre: i, len: 1, alt: None
		};
	}
	let len = lvaluelen(&horizon[i..]);
	if i + len == horizon.len() && (i > 0 || is_horizon_lengthenable) {
		return flush(i);
	}
	if len != 0 {
		return WhatNow{
			tri: Transition::Push(Box::new(SitRvalue{end_trigger})),
			pre: i + len, len: 0, alt: None
		};
	}
	let len = predlen(is_word, &horizon[i..]);
	if i + len == horizon.len() && (i > 0 || is_horizon_lengthenable) {
		return flush(i);
	}
	let word = &horizon[i..i+len];
	match word {
		b"[[" => WhatNow{
			tri: Transition::Push(Box::new(
				SitVec{terminator: vec!{b']', b']'}, color: COLOR_MAGIC}
			)),
			pre: i, len, alt: None
		},
		b"case" => WhatNow{
			tri: Transition::Push(Box::new(SitIn{})),
			pre: i, len, alt: None
		},
		b"do" |
		b"done" |
		b"elif" |
		b"else" |
		b"esac" |
		b"fi" |
		b"for" |
		b"if" |
		b"select" |
		b"then" |
		b"until" |
		b"while" |
		b"{" |
		b"}" => WhatNow{
			tri: Transition::Push(Box::new(SitExtent{
				len,
				color: COLOR_KWD,
				end_insert: None
			})), pre: i, len: 0, alt: None
		},
		_ => WhatNow{
			tri: Transition::Push(Box::new(SitCmd{end_trigger})),
			pre: i, len: 0, alt: None
		},
	}
}

pub fn common_arg_cmd(
	end_trigger :u16,
	horizon :&[u8],
	i :usize,
	is_horizon_lengthenable :bool,
) -> Option<WhatNow> {
	if let Some(res) = find_command_enders(horizon, i, is_horizon_lengthenable) {
		return Some(res);
	}
	common_no_cmd(end_trigger, horizon, i, is_horizon_lengthenable)
}

pub fn common_no_cmd(
	end_trigger :u16,
	horizon :&[u8],
	i :usize,
	is_horizon_lengthenable :bool,
) -> Option<WhatNow> {
	if let Some(res) = find_usual_suspects(end_trigger, horizon, i, is_horizon_lengthenable) {
		return Some(res);
	}
	match common_str_cmd(&horizon, i, is_horizon_lengthenable, true) {
		CommonStrCmdResult::None => None,
		CommonStrCmdResult::Some(x) => Some(x),
		CommonStrCmdResult::OnlyWithQuotes(_) => {
			Some(WhatNow{
				tri: Transition::Push(Box::new(SitStrPhantom{
					cmd_end_trigger: end_trigger,
				})), pre: i, len: 0, alt: Some(b"\"")
			})
		},
	}
}

pub fn common_quoting_unneeded(
	end_trigger :u16,
	horizon :&[u8],
	i :usize,
	is_horizon_lengthenable :bool,
) -> Option<WhatNow> {
	if let Some(res) = find_command_enders(horizon, i, is_horizon_lengthenable) {
		return Some(res);
	}
	common_no_cmd_quoting_unneeded(end_trigger, horizon, i, is_horizon_lengthenable)
}

pub fn common_no_cmd_quoting_unneeded(
	end_trigger :u16,
	horizon :&[u8],
	i :usize,
	is_horizon_lengthenable :bool,
) -> Option<WhatNow> {
	if let Some(res) = find_usual_suspects(end_trigger, horizon, i, is_horizon_lengthenable) {
		return Some(res);
	}
	match common_str_cmd(&horizon, i, is_horizon_lengthenable, false) {
		CommonStrCmdResult::None => None,
		CommonStrCmdResult::Some(x) => Some(x),
		CommonStrCmdResult::OnlyWithQuotes(x) => {
			if horizon[i] == b'`' {
				Some(WhatNow{
					tri: Transition::Push(Box::new(SitNormal{
						end_trigger: u16::from(b'`'), end_replace: None
					})), pre: i, len: 1, alt: None
				})
			} else {
				Some(x)
			}
		},
	}
}

// Does not pop on eof → Callers must use flush_or_pop
fn find_command_enders(
	horizon :&[u8],
	i :usize,
	is_horizon_lengthenable :bool,
) -> Option<WhatNow> {
	let plen = prefixlen(&horizon[i..], b">&");
	if plen == 2 {
		return Some(flush(i + 2));
	}
	if i + plen == horizon.len() && (i > 0 || is_horizon_lengthenable) {
		return Some(flush(i));
	}
	let a = horizon[i];
	if a == b'\n' || a == b';' || a == b'|' || a == b'&' {
		return Some(WhatNow{
			tri: Transition::Pop, pre: i, len: 0, alt: None
		});
	}
	None
}

fn find_usual_suspects(
	end_trigger :u16,
	horizon :&[u8],
	i :usize,
	is_horizon_lengthenable :bool,
) -> Option<WhatNow> {
	let a = horizon[i];
	if u16::from(a) == end_trigger {
		return Some(WhatNow{
			tri: Transition::Pop, pre: i, len: 0, alt: None
		});
	}
	if a == b'#' {
		return Some(WhatNow{
			tri: Transition::Push(Box::new(SitComment{})),
			pre: i, len: 1, alt: None
		});
	}
	if a == b'\'' {
		return Some(WhatNow{
			tri: Transition::Push(Box::new(SitUntilByte{
				until: b'\'', color: 0x00_ffff00, end_replace: None
			})),
			pre: i, len: 1, alt: None
		});
	}
	if a == b'\"' {
		return Some(WhatNow{
			tri: Transition::Push(Box::new(SitStrDq{})),
			pre: i, len: 1, alt: None
		});
	}
	if a == b'$' {
		if i+1 >= horizon.len() {
			if i > 0 || is_horizon_lengthenable {
				return Some(flush(i));
			}
			return None;
		}
		let b = horizon[i+1];
		if b == b'\'' {
			return Some(WhatNow{
				tri: Transition::Push(Box::new(SitStrSqEsc{})),
				pre: i, len: 2, alt: None
			});
		} else if b == b'*' {
			// $* → "$@" but not "$*" → "$@"
			let ext = Box::new(SitExtent{
				len: 0,
				color: COLOR_VAR,
				end_insert: None
			});
			return Some(WhatNow{
				tri: Transition::Push(ext),
				pre: i, len: 2, alt: Some(b"\"$@\"")
			});
		}
	}
	let (ate, delimiter) = find_heredoc(&horizon[i ..]);
	if i + ate == horizon.len() {
		if i > 0 || is_horizon_lengthenable {
			return Some(flush(i));
		}
	} else if !delimiter.is_empty() {
		return Some(WhatNow{
			tri: Transition::Push(Box::new(
				SitVec{terminator: delimiter, color: COLOR_HERE}
			)),
			pre: i, len: ate, alt: None
		});
	} else if ate > 0 {
		return Some(flush(i + ate));
	}
	None
}

fn lvaluelen(horizon: &[u8]) -> usize {
	let mut ate = identifierlen(&horizon);
	if ate != 0 {

		#[derive(Clone)]
		#[derive(Copy)]
		enum Lex {
			Ident,
			Brack,
			Pluss,
		}
		let mut state = Lex::Ident;

		while ate != horizon.len() {
			let byte :u8 = horizon[ate];
			ate += 1;

			// TODO: Recursion: Expression tracker
			match (state, byte) {
				(Lex::Ident, b'=') => break,
				(Lex::Pluss, b'=') => break,
				(Lex::Ident, b'[') => state = Lex::Brack,
				(Lex::Brack, b']') => state = Lex::Ident,
				(Lex::Ident, b'+') => state = Lex::Pluss,
				(Lex::Ident, _) => return 0,
				(Lex::Pluss, _) => return 0,
				(Lex::Brack, _) => {},
			}
		}
	}
	ate
}

fn find_heredoc(horizon: &[u8]) -> (usize, Vec<u8>) {
	let mut ate = predlen(|x| x == b'<', &horizon);
	let mut found = Vec::<u8>::new();
	if ate != 2 {
		return (ate, found);
	}
	ate += predlen(|x| x == b'-', &horizon[ate ..]);
	ate += predlen(is_whitespace, &horizon[ate ..]);

	// Lex one word.
	let herein = &horizon[ate ..];
	found.reserve(herein.len());

	#[derive(Clone)]
	#[derive(Copy)]
	enum DelimiterSyntax {
		WORD,
		WORDESC,
		SQ,
		DQ,
		DQESC,
	}
	let mut state = DelimiterSyntax::WORD;

	for byte_ref in herein {
		let byte: u8 = *byte_ref;
		state = match (state, byte) {
			(DelimiterSyntax::WORD, b' ' ) => break,
			(DelimiterSyntax::WORD, b'\n') => break,
			(DelimiterSyntax::WORD, b'\t') => break,
			(DelimiterSyntax::WORD, b'\\') => DelimiterSyntax::WORDESC,
			(DelimiterSyntax::WORD, b'\'') => DelimiterSyntax::SQ,
			(DelimiterSyntax::WORD, b'\"') => DelimiterSyntax::DQ,
			(DelimiterSyntax::SQ, b'\'') => DelimiterSyntax::WORD,
			(DelimiterSyntax::DQ, b'\"') => DelimiterSyntax::WORD,
			(DelimiterSyntax::DQ, b'\\') => DelimiterSyntax::DQESC,
			(DelimiterSyntax::WORDESC, b'\n') => DelimiterSyntax::WORD,
			(DelimiterSyntax::WORDESC, _) => {
				found.push(byte);
				DelimiterSyntax::WORD
			},
			(DelimiterSyntax::DQESC, b'\n') => DelimiterSyntax::DQ,
			(DelimiterSyntax::DQESC, _) => {
				if byte != b'\"' && byte != b'\\' {
					found.push(b'\\');
				}
				found.push(byte);
				DelimiterSyntax::DQ
			},
			(_, _) => {
				found.push(byte);
				state
			},
		};
		ate += 1;
	}
	(ate, found)
}
