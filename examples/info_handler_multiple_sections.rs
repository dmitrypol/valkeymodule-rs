use valkey_module::alloc::ValkeyAlloc;
use valkey_module::InfoContext;
use valkey_module::{valkey_module, ValkeyResult};
use valkey_module_macros::{info_command_handler, InfoSection};

#[derive(Debug, Clone, InfoSection)]
struct InfoSection1 {
    field_1: String,
}

#[derive(Debug, Clone, InfoSection)]
struct InfoSection2 {
    field_2: String,
}

#[info_command_handler]
fn add_info(ctx: &InfoContext, _for_crash_report: bool) -> ValkeyResult<()> {
    let data = InfoSection1 {
        field_1: "value1".to_owned(),
    };
    let _ = ctx.build_one_section(data)?;

    let data = InfoSection2 {
        field_2: "value2".to_owned(),
    };

    ctx.build_one_section(data)
}

//////////////////////////////////////////////////////

valkey_module! {
    name: "info_handler_multiple_sections",
    version: 1,
    allocator: (ValkeyAlloc, ValkeyAlloc),
    data_types: [],
    commands: [],
}
