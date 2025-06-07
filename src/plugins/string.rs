use crate::core::engine::Engine;
use crate::plugins::plugin::Plugin;
use crate::prelude::{Behaviour, ScriptResult};
use crate::prelude::{ObjectTypeFlags, ScriptGeneric};
use crate::types::script_memory::ScriptMemoryLocation;
use angelscript_sys::{asINT64, asQWORD};
use std::ffi::c_void;

use itoa;
use ryu;

// Constructor from &str (used from AngelScript generics)
fn construct_string(g: &ScriptGeneric) {
    let mut ptr = g.get_object().unwrap();
    ptr.set(String::new());
}

// Copy constructor for string
fn copy_construct_string(g: &ScriptGeneric) {
    let src_ptr = g.get_arg_address(0).unwrap(); // Changed from get_arg_object to get_arg_address
    let mut dest_ptr = g.get_object().unwrap();

    if src_ptr.is_null() {
        dest_ptr.set(String::new());
        return;
    }

    let source_value = src_ptr.as_ref::<String>().clone();
    dest_ptr.set(source_value);
}

// Destructor: properly drop the string
fn destruct_string(g: &ScriptGeneric) {
    let mut ptr = g.get_object().unwrap();
    // Use drop_in_place for proper cleanup
    unsafe {
        std::ptr::drop_in_place(ptr.as_ref_mut::<String>());
    }
}

fn assign_string(g: &ScriptGeneric) {
    let src = g.get_arg_address(0).unwrap();
    let mut dest = g.get_object().unwrap();
    if src.is_null() {
        return;
    }
    dest.set(src.as_ref::<String>().clone());
    // Fix: set_return_object should use set_return_address
    g.set_return_address(&mut dest).unwrap();
}

fn add_assign_string(g: &ScriptGeneric) {
    let src = g.get_arg_address(0).unwrap();
    let mut dest = g.get_object().unwrap();

    if src.is_null() {
        return;
    }

    dest.as_ref_mut::<String>().push_str(src.as_ref::<String>());
    g.set_return_address(&mut dest).unwrap();
}

fn string_equals(g: &ScriptGeneric) {
    let lhs = g.get_object().unwrap();
    let rhs = g.get_arg_address(0).unwrap();
    let equal = lhs.as_ref::<String>() == rhs.as_ref::<String>();
    g.set_return_byte(equal.into()).unwrap();
}

fn string_cmp(g: &ScriptGeneric) {
    let lhs = g.get_object().unwrap();
    let rhs = g.get_arg_address(0).unwrap();
    let lhs_str = lhs.as_ref::<String>();
    let rhs_str = rhs.as_ref::<String>();

    let result = if lhs_str < rhs_str {
        -1i32
    } else if lhs_str > rhs_str {
        1i32
    } else {
        0i32
    };

    g.set_return_dword(result as u32).unwrap();
}

fn string_add(g: &ScriptGeneric) {
    let lhs = g.get_object().unwrap();
    let rhs = g.get_arg_address(0).unwrap();
    let mut ret = g.get_address_of_return_location().unwrap();
    ret.set(lhs.as_ref::<String>().clone() + rhs.as_ref::<String>());
}

fn string_length(g: &ScriptGeneric) {
    let obj = g.get_object().unwrap();
    g.set_return_dword(obj.as_ref::<String>().len() as u32)
        .unwrap();
}

fn string_is_empty(g: &ScriptGeneric) {
    let obj = g.get_object().unwrap();
    g.set_return_byte(obj.as_ref::<String>().is_empty().into())
        .unwrap();
}

fn string_char_at(g: &ScriptGeneric) {
    let idx = g.get_arg_dword(0) as usize;
    let mut obj = g.get_object().unwrap();

    let str_ref = obj.as_ref_mut::<String>();
    if idx >= str_ref.len() {
        let ctx = Engine::get_active_context().unwrap();
        ctx.set_exception("Index out of bounds", true).unwrap();
        g.set_return_address_raw(ScriptMemoryLocation::null())
            .unwrap();
        return;
    }

    // Fix: Return reference to the byte, not mutable bytes
    unsafe {
        let byte_ptr = str_ref.as_bytes().as_ptr().add(idx) as *mut u8;
        g.set_return_address_raw(ScriptMemoryLocation::from_const(byte_ptr as *mut c_void))
            .unwrap();
    }
}

fn string_assign_bool(g: &ScriptGeneric) {
    let mut self_ptr = g.get_object().unwrap();
    let value = g.get_address_of_arg(0).unwrap();
    self_ptr.set(
        if *value.as_ref::<bool>() {
            "true"
        } else {
            "false"
        }
        .to_string(),
    );
    g.set_return_address(&mut self_ptr).unwrap();
}

fn string_add_assign_bool(g: &ScriptGeneric) {
    let mut self_ptr = g.get_object().unwrap();
    let value = g.get_address_of_arg(0).unwrap();
    let formatted = if *value.as_ref::<bool>() {
        "true"
    } else {
        "false"
    };
    self_ptr.as_ref_mut::<String>().push_str(formatted);
    g.set_return_address(&mut self_ptr).unwrap();
}

fn string_add_bool(g: &ScriptGeneric) {
    let self_ptr = g.get_object().unwrap();
    let value = g.get_address_of_arg(0).unwrap();
    let formatted = if *value.as_ref::<bool>() {
        "true"
    } else {
        "false"
    };
    let result = format!("{}{}", self_ptr.as_ref::<String>(), formatted);
    g.get_address_of_return_location().unwrap().set(result);
}

fn bool_add_string(g: &ScriptGeneric) {
    let value = g.get_address_of_arg(0).unwrap();
    let self_ptr = g.get_arg_address(1).unwrap();
    let formatted = if *value.as_ref::<bool>() {
        "true"
    } else {
        "false"
    };
    let result = format!("{}{}", formatted, self_ptr.as_ref::<String>());
    g.get_address_of_return_location().unwrap().set(result);
}

// Fixed substring function
fn string_substring(g: &ScriptGeneric) {
    let self_ptr = g.get_object().unwrap();
    let start = g.get_arg_dword(0) as usize;
    let count = g.get_arg_dword(1) as i32;

    let string = self_ptr.as_ref::<String>();
    let substring = if count < 0 {
        // If count is -1, take from start to end
        string.get(start..).unwrap_or("").to_string()
    } else {
        let end = start + count as usize;
        string.get(start..end).unwrap_or("").to_string()
    };

    g.get_address_of_return_location().unwrap().set(substring);
}

// Add missing string methods
fn string_find_first(g: &ScriptGeneric) {
    let self_ptr = g.get_object().unwrap();
    let pattern = g.get_arg_address(0).unwrap();
    let start = g.get_arg_dword(1) as usize;

    let string = self_ptr.as_ref::<String>();
    let pattern_str = pattern.as_ref::<String>();

    let result = if start < string.len() {
        string[start..]
            .find(pattern_str)
            .map(|pos| (start + pos) as i32)
            .unwrap_or(-1)
    } else {
        -1
    };

    g.set_return_dword(result as u32).unwrap();
}

fn string_insert(g: &ScriptGeneric) {
    let mut self_ptr = g.get_object().unwrap();
    let pos = g.get_arg_dword(0) as usize;
    let other = g.get_arg_address(1).unwrap();

    let string = self_ptr.as_ref_mut::<String>();
    let other_str = other.as_ref::<String>();

    if pos <= string.len() {
        string.insert_str(pos, other_str);
    }
}

fn string_erase(g: &ScriptGeneric) {
    let mut self_ptr = g.get_object().unwrap();
    let pos = g.get_arg_dword(0) as usize;
    let count = g.get_arg_dword(1) as i32;

    let string = self_ptr.as_ref_mut::<String>();

    if pos < string.len() {
        let end = if count < 0 {
            string.len()
        } else {
            (pos + count as usize).min(string.len())
        };

        string.drain(pos..end);
    }
}

// Additional string search methods
fn string_find_first_of(g: &ScriptGeneric) {
    let self_ptr = g.get_object().unwrap();
    let chars = g.get_arg_address(0).unwrap();
    let start = g.get_arg_dword(1) as usize;

    let string = self_ptr.as_ref::<String>();
    let chars_str = chars.as_ref::<String>();

    let result = if start < string.len() {
        string[start..]
            .find(|c| chars_str.contains(c))
            .map(|pos| (start + pos) as i32)
            .unwrap_or(-1)
    } else {
        -1
    };

    g.set_return_dword(result as u32).unwrap();
}

fn string_find_first_not_of(g: &ScriptGeneric) {
    let self_ptr = g.get_object().unwrap();
    let chars = g.get_arg_address(0).unwrap();
    let start = g.get_arg_dword(1) as usize;

    let string = self_ptr.as_ref::<String>();
    let chars_str = chars.as_ref::<String>();

    let result = if start < string.len() {
        string[start..]
            .find(|c| !chars_str.contains(c))
            .map(|pos| (start + pos) as i32)
            .unwrap_or(-1)
    } else {
        -1
    };

    g.set_return_dword(result as u32).unwrap();
}

fn string_find_last(g: &ScriptGeneric) {
    let self_ptr = g.get_object().unwrap();
    let pattern = g.get_arg_address(0).unwrap();
    let start = g.get_arg_dword(1) as i32;

    let string = self_ptr.as_ref::<String>();
    let pattern_str = pattern.as_ref::<String>();

    let search_range = if start < 0 {
        string.as_str()
    } else {
        let end = (start as usize + 1).min(string.len());
        &string[..end]
    };

    let result = search_range
        .rfind(pattern_str)
        .map(|pos| pos as i32)
        .unwrap_or(-1);

    g.set_return_dword(result as u32).unwrap();
}

fn string_find_last_of(g: &ScriptGeneric) {
    let self_ptr = g.get_object().unwrap();
    let chars = g.get_arg_address(0).unwrap();
    let start = g.get_arg_dword(1) as i32;

    let string = self_ptr.as_ref::<String>();
    let chars_str = chars.as_ref::<String>();

    let search_range = if start < 0 {
        string.as_str()
    } else {
        let end = (start as usize + 1).min(string.len());
        &string[..end]
    };

    let result = search_range
        .rfind(|c| chars_str.contains(c))
        .map(|pos| pos as i32)
        .unwrap_or(-1);

    g.set_return_dword(result as u32).unwrap();
}

fn string_find_last_not_of(g: &ScriptGeneric) {
    let self_ptr = g.get_object().unwrap();
    let chars = g.get_arg_address(0).unwrap();
    let start = g.get_arg_dword(1) as i32;

    let string = self_ptr.as_ref::<String>();
    let chars_str = chars.as_ref::<String>();

    let search_range = if start < 0 {
        string.as_str()
    } else {
        let end = (start as usize + 1).min(string.len());
        &string[..end]
    };

    let result = search_range
        .rfind(|c| !chars_str.contains(c))
        .map(|pos| pos as i32)
        .unwrap_or(-1);

    g.set_return_dword(result as u32).unwrap();
}

// Global parsing functions
fn parse_int(g: &ScriptGeneric) {
    let string_ptr = g.get_arg_address(0).unwrap();
    let base = g.get_arg_dword(1);
    let byte_count_ptr = g.get_arg_address(2); // Optional out parameter

    let string = string_ptr.as_ref::<String>();
    let trimmed = string.trim();

    let (result, bytes_parsed) = match i64::from_str_radix(trimmed, base) {
        Ok(val) => (val, trimmed.len()),
        Err(_) => {
            // Try to parse as much as possible
            let mut chars = trimmed.chars();
            let mut valid_str = String::new();

            // Handle sign
            if let Some(first) = chars.next() {
                if first == '-' || first == '+' {
                    valid_str.push(first);
                }
            }

            // Parse digits
            for ch in chars {
                if ch.is_digit(base) {
                    valid_str.push(ch);
                } else {
                    break;
                }
            }

            match i64::from_str_radix(&valid_str, base) {
                Ok(val) => (val, valid_str.len()),
                Err(_) => (0, 0),
            }
        }
    };

    // Set byte count if provided
    if let Some(mut byte_count_ptr) = byte_count_ptr {
        if !byte_count_ptr.is_null() {
            byte_count_ptr
                .as_ref_mut::<u32>()
                .clone_from(&(bytes_parsed as u32));
        }
    }

    g.set_return_qword(result as u64).unwrap();
}

fn parse_uint(g: &ScriptGeneric) {
    let string_ptr = g.get_arg_address(0).unwrap();
    let base = g.get_arg_dword(1);
    let byte_count_ptr = g.get_arg_address(2); // Optional out parameter

    let string = string_ptr.as_ref::<String>();
    let trimmed = string.trim();

    let (result, bytes_parsed) = match u64::from_str_radix(trimmed, base) {
        Ok(val) => (val, trimmed.len()),
        Err(_) => {
            // Try to parse as much as possible
            let mut valid_str = String::new();

            for ch in trimmed.chars() {
                if ch.is_digit(base) {
                    valid_str.push(ch);
                } else {
                    break;
                }
            }

            match u64::from_str_radix(&valid_str, base) {
                Ok(val) => (val, valid_str.len()),
                Err(_) => (0, 0),
            }
        }
    };

    // Set byte count if provided
    if let Some(mut byte_count_ptr) = byte_count_ptr {
        if !byte_count_ptr.is_null() {
            byte_count_ptr
                .as_ref_mut::<u32>()
                .clone_from(&(bytes_parsed as u32));
        }
    }

    g.set_return_qword(result).unwrap();
}

fn parse_float(g: &ScriptGeneric) {
    let string_ptr = g.get_arg_address(0).unwrap();
    let byte_count_ptr = g.get_arg_address(1); // Optional out parameter

    let string = string_ptr.as_ref::<String>();
    let trimmed = string.trim();

    let (result, bytes_parsed) = match trimmed.parse::<f64>() {
        Ok(val) => (val, trimmed.len()),
        Err(_) => {
            // Try to parse as much as possible
            let mut valid_str = String::new();
            let mut has_dot = false;
            let mut has_e = false;

            for (i, ch) in trimmed.chars().enumerate() {
                match ch {
                    '0'..='9' => valid_str.push(ch),
                    '.' if !has_dot && !has_e => {
                        has_dot = true;
                        valid_str.push(ch);
                    }
                    'e' | 'E' if !has_e && i > 0 => {
                        has_e = true;
                        valid_str.push(ch);
                    }
                    '-' | '+' if i == 0 || (has_e && valid_str.ends_with(['e', 'E'])) => {
                        valid_str.push(ch);
                    }
                    _ => break,
                }
            }

            match valid_str.parse::<f64>() {
                Ok(val) => (val, valid_str.len()),
                Err(_) => (0.0, 0),
            }
        }
    };

    // Set byte count if provided
    if let Some(mut byte_count_ptr) = byte_count_ptr {
        if !byte_count_ptr.is_null() {
            byte_count_ptr
                .as_ref_mut::<u32>()
                .clone_from(&(bytes_parsed as u32));
        }
    }

    g.set_return_double(result).unwrap();
}

// Updated formatting functions using ryu and itoa
fn format_int(g: &ScriptGeneric) {
    let val = g.get_arg_qword(0) as i64;
    let options = g.get_arg_address(1).unwrap();
    let width = g.get_arg_dword(2) as usize;

    let options_str = options.as_ref::<String>();

    let result = if options_str.contains('x') || options_str.contains('X') {
        if options_str.contains('X') {
            format!("{:0width$X}", val, width = width)
        } else {
            format!("{:0width$x}", val, width = width)
        }
    } else if options_str.contains('o') {
        format!("{:0width$o}", val, width = width)
    } else if options_str.contains('b') {
        format!("{:0width$b}", val, width = width)
    } else {
        // Use itoa for fast decimal formatting
        let mut buffer = itoa::Buffer::new();
        let formatted = buffer.format(val);

        if width > formatted.len() {
            format!("{:0width$}", formatted, width = width)
        } else {
            formatted.to_string()
        }
    };

    g.get_address_of_return_location().unwrap().set(result);
}

fn format_uint(g: &ScriptGeneric) {
    let val = g.get_arg_qword(0);
    let options = g.get_arg_address(1).unwrap();
    let width = g.get_arg_dword(2) as usize;

    let options_str = options.as_ref::<String>();

    let result = if options_str.contains('x') || options_str.contains('X') {
        if options_str.contains('X') {
            format!("{:0width$X}", val, width = width)
        } else {
            format!("{:0width$x}", val, width = width)
        }
    } else if options_str.contains('o') {
        format!("{:0width$o}", val, width = width)
    } else if options_str.contains('b') {
        format!("{:0width$b}", val, width = width)
    } else {
        // Use itoa for fast decimal formatting
        let mut buffer = itoa::Buffer::new();
        let formatted = buffer.format(val);

        if width > formatted.len() {
            format!("{:0width$}", formatted, width = width)
        } else {
            formatted.to_string()
        }
    };

    g.get_address_of_return_location().unwrap().set(result);
}

fn format_float(g: &ScriptGeneric) {
    let val = g.get_arg_double(0);
    let options = g.get_arg_address(1).unwrap();
    let width = g.get_arg_dword(2) as usize;
    let precision = g.get_arg_dword(3) as usize;

    let options_str = options.as_ref::<String>();

    let result = if options_str.contains('e') || options_str.contains('E') {
        // Use standard formatting for scientific notation
        if options_str.contains('E') {
            format!(
                "{:0width$.precision$E}",
                val,
                width = width,
                precision = precision
            )
        } else {
            format!(
                "{:0width$.precision$e}",
                val,
                width = width,
                precision = precision
            )
        }
    } else if precision > 0 {
        // Use standard formatting when precision is specified
        format!(
            "{:0width$.precision$}",
            val,
            width = width,
            precision = precision
        )
    } else {
        // Use ryu for fast default float formatting
        let mut buffer = ryu::Buffer::new();
        let formatted = buffer.format(val);

        if width > formatted.len() {
            format!("{:0width$}", formatted, width = width)
        } else {
            formatted.to_string()
        }
    };

    g.get_address_of_return_location().unwrap().set(result);
}

// Also update the primitive assignment and addition functions to use itoa/ryu
fn string_assign_int(g: &ScriptGeneric) {
    let mut self_ptr = g.get_object().unwrap();
    let value = g.get_address_of_arg(0).unwrap();

    // Use itoa for fast conversion
    let mut buffer = itoa::Buffer::new();
    let formatted = buffer.format(*value.as_ref::<asINT64>());

    self_ptr.set(formatted.to_string());
    g.set_return_address(&mut self_ptr).unwrap();
}

fn string_assign_uint(g: &ScriptGeneric) {
    let mut self_ptr = g.get_object().unwrap();
    let value = g.get_address_of_arg(0).unwrap();

    // Use itoa for fast conversion
    let mut buffer = itoa::Buffer::new();
    let formatted = buffer.format(*value.as_ref::<asQWORD>());

    self_ptr.set(formatted.to_string());
    g.set_return_address(&mut self_ptr).unwrap();
}

fn string_assign_double(g: &ScriptGeneric) {
    let mut self_ptr = g.get_object().unwrap();
    let value = g.get_address_of_arg(0).unwrap();

    // Use ryu for fast conversion
    let mut buffer = ryu::Buffer::new();
    let formatted = buffer.format(*value.as_ref::<f64>());

    self_ptr.set(formatted.to_string());
    g.set_return_address(&mut self_ptr).unwrap();
}

fn string_assign_float(g: &ScriptGeneric) {
    let mut self_ptr = g.get_object().unwrap();
    let value = g.get_address_of_arg(0).unwrap();

    // Use ryu for fast conversion
    let mut buffer = ryu::Buffer::new();
    let formatted = buffer.format(*value.as_ref::<f32>());

    self_ptr.set(formatted.to_string());
    g.set_return_address(&mut self_ptr).unwrap();
}

// Update add-assign functions
fn string_add_assign_int(g: &ScriptGeneric) {
    let mut self_ptr = g.get_object().unwrap();
    let value = g.get_address_of_arg(0).unwrap();

    let mut buffer = itoa::Buffer::new();
    let formatted = buffer.format(*value.as_ref::<asINT64>());

    self_ptr.as_ref_mut::<String>().push_str(formatted);
    g.set_return_address(&mut self_ptr).unwrap();
}

fn string_add_assign_uint(g: &ScriptGeneric) {
    let mut self_ptr = g.get_object().unwrap();
    let value = g.get_address_of_arg(0).unwrap();

    let mut buffer = itoa::Buffer::new();
    let formatted = buffer.format(*value.as_ref::<asQWORD>());

    self_ptr.as_ref_mut::<String>().push_str(formatted);
    g.set_return_address(&mut self_ptr).unwrap();
}

fn string_add_assign_double(g: &ScriptGeneric) {
    let mut self_ptr = g.get_object().unwrap();
    let value = g.get_address_of_arg(0).unwrap();

    let mut buffer = ryu::Buffer::new();
    let formatted = buffer.format(*value.as_ref::<f64>());

    self_ptr.as_ref_mut::<String>().push_str(formatted);
    g.set_return_address(&mut self_ptr).unwrap();
}

fn string_add_assign_float(g: &ScriptGeneric) {
    let mut self_ptr = g.get_object().unwrap();
    let value = g.get_address_of_arg(0).unwrap();

    let mut buffer = ryu::Buffer::new();
    let formatted = buffer.format(*value.as_ref::<f32>());

    self_ptr.as_ref_mut::<String>().push_str(formatted);
    g.set_return_address(&mut self_ptr).unwrap();
}

// Update string addition functions
fn string_add_int(g: &ScriptGeneric) {
    let self_ptr = g.get_object().unwrap();
    let value = g.get_address_of_arg(0).unwrap();

    let mut buffer = itoa::Buffer::new();
    let formatted = buffer.format(*value.as_ref::<asINT64>());

    let result = format!("{}{}", self_ptr.as_ref::<String>(), formatted);
    g.get_address_of_return_location().unwrap().set(result);
}

fn int_add_string(g: &ScriptGeneric) {
    let value = g.get_address_of_arg(0).unwrap();
    let self_ptr = g.get_arg_address(1).unwrap();

    let mut buffer = itoa::Buffer::new();
    let formatted = buffer.format(*value.as_ref::<asINT64>());

    let result = format!("{}{}", formatted, self_ptr.as_ref::<String>());
    g.get_address_of_return_location().unwrap().set(result);
}

fn string_add_uint(g: &ScriptGeneric) {
    let self_ptr = g.get_object().unwrap();
    let value = g.get_address_of_arg(0).unwrap();

    let mut buffer = itoa::Buffer::new();
    let formatted = buffer.format(*value.as_ref::<asQWORD>());

    let result = format!("{}{}", self_ptr.as_ref::<String>(), formatted);
    g.get_address_of_return_location().unwrap().set(result);
}

fn uint_add_string(g: &ScriptGeneric) {
    let value = g.get_address_of_arg(0).unwrap();
    let self_ptr = g.get_arg_address(1).unwrap();

    let mut buffer = itoa::Buffer::new();
    let formatted = buffer.format(*value.as_ref::<asQWORD>());

    let result = format!("{}{}", formatted, self_ptr.as_ref::<String>());
    g.get_address_of_return_location().unwrap().set(result);
}

fn string_add_double(g: &ScriptGeneric) {
    let self_ptr = g.get_object().unwrap();
    let value = g.get_address_of_arg(0).unwrap();

    let mut buffer = ryu::Buffer::new();
    let formatted = buffer.format(*value.as_ref::<f64>());

    let result = format!("{}{}", self_ptr.as_ref::<String>(), formatted);
    g.get_address_of_return_location().unwrap().set(result);
}

fn double_add_string(g: &ScriptGeneric) {
    let value = g.get_address_of_arg(0).unwrap();
    let self_ptr = g.get_arg_address(1).unwrap();

    let mut buffer = ryu::Buffer::new();
    let formatted = buffer.format(*value.as_ref::<f64>());

    let result = format!("{}{}", formatted, self_ptr.as_ref::<String>());
    g.get_address_of_return_location().unwrap().set(result);
}

fn string_add_float(g: &ScriptGeneric) {
    let self_ptr = g.get_object().unwrap();
    let value = g.get_address_of_arg(0).unwrap();

    let mut buffer = ryu::Buffer::new();
    let formatted = buffer.format(*value.as_ref::<f32>());

    let result = format!("{}{}", self_ptr.as_ref::<String>(), formatted);
    g.get_address_of_return_location().unwrap().set(result);
}

fn float_add_string(g: &ScriptGeneric) {
    let value = g.get_address_of_arg(0).unwrap();
    let self_ptr = g.get_arg_address(1).unwrap();

    let mut buffer = ryu::Buffer::new();
    let formatted = buffer.format(*value.as_ref::<f32>());

    let result = format!("{}{}", formatted, self_ptr.as_ref::<String>());
    g.get_address_of_return_location().unwrap().set(result);
}

/// Create a complete string plugin for AngelScript
pub fn plugin() -> ScriptResult<Plugin> {
    let plugin = Plugin::new()
        .ty::<String>("string", |ctx| {
            ctx.as_value_type()
               .with_flags(ObjectTypeFlags::VALUE | ObjectTypeFlags::APP_CLASS_CDAK)
                // Constructors
               .with_behavior(Behaviour::Construct, "void f()", construct_string, None, None, None)
               .with_behavior(Behaviour::Construct, "void f(const string &in)", copy_construct_string, None, None, None)
               .with_behavior(Behaviour::Destruct, "void f()", destruct_string, None, None, None)

                // Assignment operators
               .with_method("string &opAssign(const string &in)", assign_string, None, None, None)
               .with_method("string &opAddAssign(const string &in)", add_assign_string, None, None, None)

                // Comparison operators
               .with_method("bool opEquals(const string &in) const", string_equals, None, None, None)
               .with_method("int opCmp(const string &in) const", string_cmp, None, None, None)

                // String concatenation
               .with_method("string opAdd(const string &in) const", string_add, None, None, None)

                // Basic methods
               .with_method("uint length() const", string_length, None, None, None)
               .with_method("bool isEmpty() const", string_is_empty, None, None, None)
               .with_method("uint8 &opIndex(uint)", string_char_at, None, None, None)
               .with_method("uint8 &opIndex(uint) const", string_char_at, None, None, None)

                // Primitive type operations
               .with_method("string &opAssign(double)", string_assign_double, None, None, None)
               .with_method("string &opAddAssign(double)", string_add_assign_double, None, None, None)
               .with_method("string opAdd(double) const", string_add_double, None, None, None)
               .with_method("string opAdd_r(double) const", double_add_string, None, None, None)

               .with_method("string &opAssign(float)", string_assign_float, None, None, None)
               .with_method("string &opAddAssign(float)", string_add_assign_float, None, None, None)
               .with_method("string opAdd(float) const", string_add_float, None, None, None)
               .with_method("string opAdd_r(float) const", float_add_string, None, None, None)

               .with_method("string &opAssign(int64)", string_assign_int, None, None, None)
               .with_method("string &opAddAssign(int64)", string_add_assign_int, None, None, None)
               .with_method("string opAdd(int64) const", string_add_int, None, None, None)
               .with_method("string opAdd_r(int64) const", int_add_string, None, None, None)

               .with_method("string &opAssign(uint64)", string_assign_uint, None, None, None)
               .with_method("string &opAddAssign(uint64)", string_add_assign_uint, None, None, None)
               .with_method("string opAdd(uint64) const", string_add_uint, None, None, None)
               .with_method("string opAdd_r(uint64) const", uint_add_string, None, None, None)

               .with_method("string &opAssign(bool)", string_assign_bool, None, None, None)
               .with_method("string &opAddAssign(bool)", string_add_assign_bool, None, None, None)
               .with_method("string opAdd(bool) const", string_add_bool, None, None, None)
               .with_method("string opAdd_r(bool) const", bool_add_string, None, None, None)

                // String manipulation methods
               .with_method("string substr(uint start = 0, int count = -1) const", string_substring, None, None, None)
               .with_method("int findFirst(const string &in, uint start = 0) const", string_find_first, None, None, None)
               .with_method("int findFirstOf(const string &in, uint start = 0) const", string_find_first_of, None, None, None)
               .with_method("int findFirstNotOf(const string &in, uint start = 0) const", string_find_first_not_of, None, None, None)
               .with_method("int findLast(const string &in, int start = -1) const", string_find_last, None, None, None)
               .with_method("int findLastOf(const string &in, int start = -1) const", string_find_last_of, None, None, None)
               .with_method("int findLastNotOf(const string &in, int start = -1) const", string_find_last_not_of, None, None, None)
               .with_method("void insert(uint pos, const string &in)", string_insert, None, None, None)
               .with_method("void erase(uint pos, int count = -1)", string_erase, None, None, None);
        })
        // Global formatting and parsing functions
        .function("string formatInt(int64 val, const string &in options = \"\", uint width = 0)", format_int, None)
        .function("string formatUInt(uint64 val, const string &in options = \"\", uint width = 0)", format_uint, None)
        .function("string formatFloat(double val, const string &in options = \"\", uint width = 0, uint precision = 0)", format_float, None)
        .function("int64 parseInt(const string &in, uint base = 10, uint &out byteCount = 0)", parse_int, None)
        .function("uint64 parseUInt(const string &in, uint base = 10, uint &out byteCount = 0)", parse_uint, None)
        .function("double parseFloat(const string &in, uint &out byteCount = 0)", parse_float, None);

    Ok(plugin)
}
