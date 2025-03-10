use crate::{
    Context, RedisModuleCommandFilter, RedisModuleCommandFilterCtx, RedisModuleString,
    RedisModule_CommandFilterArgDelete, RedisModule_CommandFilterArgGet,
    RedisModule_CommandFilterArgInsert, RedisModule_CommandFilterArgReplace,
    RedisModule_CommandFilterArgsCount, RedisModule_CommandFilterGetClientId,
    RedisModule_RegisterCommandFilter, RedisModule_UnregisterCommandFilter, ValkeyString,
};
use std::ffi::c_int;
use std::str::Utf8Error;

#[derive(Debug, Clone, Copy)]
pub struct CommandFilter {
    inner: *mut RedisModuleCommandFilter,
}

// otherwise you get error:     cannot be shared between threads safely
unsafe impl Send for CommandFilter {}
unsafe impl Sync for CommandFilter {}

impl CommandFilter {
    pub fn is_null(&self) -> bool {
        self.inner.is_null()
    }

    pub fn args_count(ctx: *mut RedisModuleCommandFilterCtx) -> c_int {
        unsafe { RedisModule_CommandFilterArgsCount.unwrap()(ctx) }
    }

    pub fn arg_get(ctx: *mut RedisModuleCommandFilterCtx, pos: c_int) -> *mut RedisModuleString {
        unsafe { RedisModule_CommandFilterArgGet.unwrap()(ctx, pos) }
    }

    /// wrapper to get argument as a &str instead of RedisModuleString
    pub fn arg_get_as_str<'a>(
        ctx: *mut RedisModuleCommandFilterCtx,
        pos: c_int,
    ) -> Result<&'a str, Utf8Error> {
        let arg = CommandFilter::arg_get(ctx, pos);
        ValkeyString::from_ptr(arg)
    }

    pub fn arg_replace(
        ctx: *mut RedisModuleCommandFilterCtx,
        pos: c_int,
        arg: *mut RedisModuleString,
    ) {
        unsafe { RedisModule_CommandFilterArgReplace.unwrap()(ctx, pos, arg) };
    }

    pub fn arg_insert(
        ctx: *mut RedisModuleCommandFilterCtx,
        pos: c_int,
        arg: *mut RedisModuleString,
    ) {
        unsafe { RedisModule_CommandFilterArgInsert.unwrap()(ctx, pos, arg) };
    }

    pub fn arg_delete(ctx: *mut RedisModuleCommandFilterCtx, pos: c_int) {
        unsafe { RedisModule_CommandFilterArgDelete.unwrap()(ctx, pos) };
    }

    pub fn get_client_id(ctx: *mut RedisModuleCommandFilterCtx) -> u64 {
        unsafe { RedisModule_CommandFilterGetClientId.unwrap()(ctx) }
    }
}

impl Context {
    pub fn register_command_filter(
        &self,
        module_cmd_filter_func: extern "C" fn(*mut RedisModuleCommandFilterCtx),
        flags: c_int,
    ) -> CommandFilter {
        let module_cmd_filter = unsafe {
            RedisModule_RegisterCommandFilter.unwrap()(
                self.ctx,
                Some(module_cmd_filter_func),
                flags,
            )
        };
        CommandFilter {
            inner: module_cmd_filter,
        }
    }
    pub fn unregister_command_filter(&self, cmd_filter: &CommandFilter) {
        unsafe {
            RedisModule_UnregisterCommandFilter.unwrap()(self.ctx, cmd_filter.inner);
        }
    }
}
