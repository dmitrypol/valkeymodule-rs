use crate::{raw, Context, RedisModuleCommandFilterCtx, REDISMODULE_CMDFILTER_NOSELF};
use std::ffi::c_int;
use std::ptr;

#[derive(Debug)]
pub struct CommandFilter {
    pub(crate) inner: *mut raw::RedisModuleCommandFilter,
}

unsafe impl Send for CommandFilter {}

impl Drop for CommandFilter {
    fn drop(&mut self) {
        unsafe { raw::RedisModule_UnregisterCommandFilter.unwrap()(ptr::null_mut(), self.inner) };
    }
}
impl Context {
    #[must_use]
    pub fn register_command_filter(&self, callback: fn(CommandFilter)) -> CommandFilter {
        // unsafe extern "C" fn filter(filter_ctx: *mut RedisModuleCommandFilterCtx) {
        //     filter_fn(CommandFilter { inner: filter_ctx });
        // }
        unsafe extern "C" fn rm_cmd_filter_func(
            rm_cmd_filter_ctx: *mut RedisModuleCommandFilterCtx,
        ) {
            if rm_cmd_filter_ctx.is_null() {
                return;
            }
        }
        let rm_cmd_filter = unsafe {
            raw::RedisModule_RegisterCommandFilter.unwrap()(
                self.ctx,
                Some(rm_cmd_filter_func),
                REDISMODULE_CMDFILTER_NOSELF as c_int,
            )
        };
        callback(CommandFilter {
            inner: rm_cmd_filter,
        });
        CommandFilter {
            inner: rm_cmd_filter,
        }
    }
}
