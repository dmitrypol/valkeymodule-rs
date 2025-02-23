use valkey_module::alloc::ValkeyAlloc;
use valkey_module::filter::CommandFilter;
use valkey_module::{valkey_module, Context, Status, ValkeyString};

fn init(ctx: &Context, _args: &[ValkeyString]) -> Status {
    let callback = |cf: CommandFilter| {
        ctx.log_notice(format!("filter: {:?}", cf));
    };
    let cmd_filter = ctx.register_command_filter(callback);
    ctx.log_notice(format!("filter: {:?}", cmd_filter));
    if cmd_filter == None {
        return Status::Err;
    }
    Status::Ok
}

// unsafe extern "C" fn filter(_filter_ctx: *mut raw::RedisModuleCommandFilterCtx) {
//     // TODO
// }

valkey_module! {
    name: "filter",
    version: 1,
    allocator: (ValkeyAlloc, ValkeyAlloc),
    data_types: [],
    init: init,
    commands: [
    ],
}
