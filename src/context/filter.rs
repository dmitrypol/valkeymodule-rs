use crate::raw;
use crate::Context;

pub struct CommandFilter {
    pub(crate) inner: *mut raw::RedisModuleCommandFilter,
}

unsafe impl Send for CommandFilter {}

// impl Drop for CommandFilter {
//     fn drop(&mut self) {
//         unsafe { raw::RedisModuleCommandFilter.unwrap()(self.inner, ptr::null_mut()) };
//     }
// }

unsafe extern "C" fn filter(filter: *mut raw::RedisModuleCommandFilterCtx){
    // This is a placeholder function that does nothing
}

impl Context {
    #[must_use]
    pub fn register_command_filter(&self) -> CommandFilter {
        let command_filter =
            unsafe { raw::RedisModule_RegisterCommandFilter.unwrap()(self.ctx, Some(filter), 0) };

        CommandFilter {
            inner: command_filter,
        }
    }
}
