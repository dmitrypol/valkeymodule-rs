use crate::{raw, CommandFilterCtx, RedisModuleCommandFilterCtx, RedisModuleString, ValkeyString};
use libc::c_int;
use std::ptr::null_mut;

impl CommandFilterCtx {
    pub fn test(args: &[&str]) -> Self {
        super::setup_test_shims();
        let args = args
            .iter()
            .map(|arg| ValkeyString::test(*arg).take())
            .collect::<Vec<_>>();
        let inner = Box::into_raw(Box::new(args)).cast::<RedisModuleCommandFilterCtx>();
        CommandFilterCtx::new(inner)
    }
}

// CommandFilterCtx::test stores fake filter args as Vec<RedisModuleString>
// behind the RedisModuleCommandFilterCtx pointer type. Cast it back for shims.
fn test_filter_args<'a>(fctx: *mut RedisModuleCommandFilterCtx) -> &'a Vec<*mut RedisModuleString> {
    unsafe { &*fctx.cast::<Vec<*mut RedisModuleString>>() }
}

fn test_filter_args_mut<'a>(
    fctx: *mut RedisModuleCommandFilterCtx,
) -> &'a mut Vec<*mut RedisModuleString> {
    unsafe { &mut *fctx.cast::<Vec<*mut RedisModuleString>>() }
}

pub(super) extern "C" fn test_command_filter_args_count(
    fctx: *mut RedisModuleCommandFilterCtx,
) -> c_int {
    test_filter_args(fctx).len() as c_int
}

pub(super) extern "C" fn test_command_filter_arg_get(
    fctx: *mut RedisModuleCommandFilterCtx,
    pos: c_int,
) -> *mut RedisModuleString {
    test_filter_args(fctx)
        .get(pos as usize)
        .copied()
        .unwrap_or(null_mut())
}

pub(super) extern "C" fn test_command_filter_arg_replace(
    fctx: *mut RedisModuleCommandFilterCtx,
    pos: c_int,
    arg: *mut RedisModuleString,
) -> c_int {
    let args = test_filter_args_mut(fctx);
    if let Some(current) = args.get_mut(pos as usize) {
        *current = arg;
        raw::Status::Ok as c_int
    } else {
        raw::Status::Err as c_int
    }
}

pub(super) extern "C" fn test_command_filter_arg_insert(
    fctx: *mut RedisModuleCommandFilterCtx,
    pos: c_int,
    arg: *mut RedisModuleString,
) -> c_int {
    let args = test_filter_args_mut(fctx);
    let pos = pos as usize;
    if pos <= args.len() {
        args.insert(pos, arg);
        raw::Status::Ok as c_int
    } else {
        raw::Status::Err as c_int
    }
}

pub(super) extern "C" fn test_command_filter_arg_delete(
    fctx: *mut RedisModuleCommandFilterCtx,
    pos: c_int,
) -> c_int {
    let args = test_filter_args_mut(fctx);
    let pos = pos as usize;
    if pos < args.len() {
        args.remove(pos);
        raw::Status::Ok as c_int
    } else {
        raw::Status::Err as c_int
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_command_filter_args_count() {
        let ctx = CommandFilterCtx::test(&["SET", "key", "value"]);
        assert_eq!(ctx.args_count(), 3);
    }

    #[test]
    fn test_command_filter_arg_get() {
        let ctx = CommandFilterCtx::test(&["SET", "key", "value"]);
        let cmd = ctx.arg_get(0);
        assert_eq!(ValkeyString::from_ptr(cmd).unwrap(), "SET");
        let arg1 = ctx.arg_get(1);
        assert_eq!(ValkeyString::from_ptr(arg1).unwrap(), "key");
    }

    #[test]
    fn test_command_filter_arg_replace() {
        let ctx = CommandFilterCtx::test(&["SET", "key", "value"]);

        ctx.arg_replace(2, "new-value");

        assert_eq!(ctx.arg_get_try_as_str(2).unwrap(), "new-value");
    }

    #[test]
    fn test_command_filter_arg_insert() {
        let ctx = CommandFilterCtx::test(&["SET", "key", "value"]);

        ctx.arg_insert(2, "EX");

        assert_eq!(ctx.args_count(), 4);
        assert_eq!(ctx.arg_get_try_as_str(2).unwrap(), "EX");
        assert_eq!(ctx.arg_get_try_as_str(3).unwrap(), "value");
    }

    #[test]
    fn test_command_filter_arg_delete() {
        let ctx = CommandFilterCtx::test(&["SET", "key", "value"]);

        ctx.arg_delete(1);

        assert_eq!(ctx.args_count(), 2);
        assert_eq!(ctx.arg_get_try_as_str(1).unwrap(), "value");
    }
}
