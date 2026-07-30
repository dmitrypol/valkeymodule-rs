use super::expectation::{Expectation0, Expectation1, Expectation2, MethodMock};
use crate::{raw, Context, ValkeyString};
use libc::{c_char, c_void};
use std::any::Any;
use std::cell::RefCell;
use std::collections::HashSet;
use std::marker::PhantomData;
use std::ops::Deref;
use std::panic::{catch_unwind, resume_unwind, AssertUnwindSafe};
use std::ptr::null_mut;
use std::rc::Rc;
use std::sync::{Mutex, OnceLock};

struct ContextExpectations {
    get_client_id: MethodMock<(), u64>,
    get_client_name_by_id: MethodMock<u64, crate::ValkeyResult<ValkeyString>>,
    set_client_name_by_id: MethodMock<(u64, Vec<u8>), raw::Status>,
    get_client_username_by_id: MethodMock<u64, crate::ValkeyResult<ValkeyString>>,
    get_client_cert: MethodMock<(), crate::ValkeyResult<ValkeyString>>,
    deauthenticate_and_close_client_by_id: MethodMock<u64, raw::Status>,
    get_current_user: MethodMock<(), ValkeyString>,
    set_module_options: MethodMock<raw::ModuleOptions, ()>,
    authenticate_client_with_acl_user: MethodMock<Vec<u8>, raw::Status>,
}

pub(crate) struct TestContextState {
    expectations: RefCell<ContextExpectations>,
    pending_panic: RefCell<Option<Box<dyn Any + Send>>>,
}

pub struct TestContext {
    context: Context,
    state: Box<TestContextState>,
    not_send_or_sync: PhantomData<Rc<()>>,
}

fn test_context_registry() -> &'static Mutex<HashSet<usize>> {
    static TEST_CONTEXTS: OnceLock<Mutex<HashSet<usize>>> = OnceLock::new();
    TEST_CONTEXTS.get_or_init(|| Mutex::new(HashSet::new()))
}

fn with_test_context_registry<T>(callback: impl FnOnce(&mut HashSet<usize>) -> T) -> T {
    let mut registry = test_context_registry()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    callback(&mut registry)
}

fn register_test_context(ctx: *mut raw::RedisModuleCtx) {
    with_test_context_registry(|registry| {
        registry.insert(ctx as usize);
    });
}

fn unregister_test_context(ctx: *mut raw::RedisModuleCtx) {
    with_test_context_registry(|registry| {
        registry.remove(&(ctx as usize));
    });
}

fn is_test_context(ctx: *mut raw::RedisModuleCtx) -> bool {
    !ctx.is_null() && with_test_context_registry(|registry| registry.contains(&(ctx as usize)))
}

fn test_context_state<'a>(ctx: *mut raw::RedisModuleCtx) -> &'a TestContextState {
    assert!(
        is_test_context(ctx),
        "context pointer was not created by Context::test"
    );
    // Context::test registers a stable Box<TestContextState> allocation under
    // this opaque pointer. The registration remains live until TestContext drops.
    unsafe { &*ctx.cast::<TestContextState>() }
}

impl Context {
    pub fn test() -> TestContext {
        super::setup_test_shims();
        let state = Box::new(TestContextState {
            expectations: RefCell::new(ContextExpectations {
                get_client_id: MethodMock::new("get_client_id"),
                get_client_name_by_id: MethodMock::new("get_client_name_by_id"),
                set_client_name_by_id: MethodMock::new("set_client_name_by_id"),
                get_client_username_by_id: MethodMock::new("get_client_username_by_id"),
                get_client_cert: MethodMock::new("get_client_cert"),
                deauthenticate_and_close_client_by_id: MethodMock::new(
                    "deauthenticate_and_close_client_by_id",
                ),
                get_current_user: MethodMock::new("get_current_user"),
                set_module_options: MethodMock::new("set_module_options"),
                authenticate_client_with_acl_user: MethodMock::new(
                    "authenticate_client_with_acl_user",
                ),
            }),
            pending_panic: RefCell::new(None),
        });
        let ctx = (&*state as *const TestContextState)
            .cast_mut()
            .cast::<raw::RedisModuleCtx>();
        register_test_context(ctx);
        TestContext {
            context: Context::new(ctx),
            state,
            not_send_or_sync: PhantomData,
        }
    }
}

impl Deref for TestContext {
    type Target = Context;

    fn deref(&self) -> &Self::Target {
        &self.context
    }
}

impl TestContext {
    pub fn expect_get_client_id(&mut self) -> Expectation0<'_, u64> {
        let expectation = self.state.expectations.get_mut().get_client_id.expect();
        Expectation0::new(expectation)
    }

    pub fn expect_get_client_name_by_id(
        &mut self,
    ) -> Expectation1<'_, u64, crate::ValkeyResult<ValkeyString>> {
        let expectation = self
            .state
            .expectations
            .get_mut()
            .get_client_name_by_id
            .expect();
        Expectation1::new(expectation)
    }

    pub fn expect_set_client_name_by_id(&mut self) -> Expectation2<'_, u64, Vec<u8>, raw::Status> {
        let expectation = self
            .state
            .expectations
            .get_mut()
            .set_client_name_by_id
            .expect();
        Expectation2::new(expectation)
    }

    pub fn expect_get_client_username_by_id(
        &mut self,
    ) -> Expectation1<'_, u64, crate::ValkeyResult<ValkeyString>> {
        let expectation = self
            .state
            .expectations
            .get_mut()
            .get_client_username_by_id
            .expect();
        Expectation1::new(expectation)
    }

    pub fn expect_get_client_cert(
        &mut self,
    ) -> Expectation0<'_, crate::ValkeyResult<ValkeyString>> {
        let expectation = self.state.expectations.get_mut().get_client_cert.expect();
        Expectation0::new(expectation)
    }

    pub fn expect_deauthenticate_and_close_client_by_id(
        &mut self,
    ) -> Expectation1<'_, u64, raw::Status> {
        let expectation = self
            .state
            .expectations
            .get_mut()
            .deauthenticate_and_close_client_by_id
            .expect();
        Expectation1::new(expectation)
    }

    pub fn expect_get_current_user(&mut self) -> Expectation0<'_, ValkeyString> {
        let expectation = self.state.expectations.get_mut().get_current_user.expect();
        Expectation0::new(expectation)
    }

    pub fn expect_set_module_options(&mut self) -> Expectation1<'_, raw::ModuleOptions, ()> {
        let expectation = self
            .state
            .expectations
            .get_mut()
            .set_module_options
            .expect();
        Expectation1::new(expectation)
    }

    pub fn expect_authenticate_client_with_acl_user(
        &mut self,
    ) -> Expectation1<'_, Vec<u8>, raw::Status> {
        let expectation = self
            .state
            .expectations
            .get_mut()
            .authenticate_client_with_acl_user
            .expect();
        Expectation1::new(expectation)
    }

    pub fn checkpoint(&mut self) {
        let expectations = self.state.expectations.get_mut();
        expectations.get_client_id.checkpoint();
        expectations.get_client_name_by_id.checkpoint();
        expectations.set_client_name_by_id.checkpoint();
        expectations.get_client_username_by_id.checkpoint();
        expectations.get_client_cert.checkpoint();
        expectations
            .deauthenticate_and_close_client_by_id
            .checkpoint();
        expectations.get_current_user.checkpoint();
        expectations.set_module_options.checkpoint();
        expectations.authenticate_client_with_acl_user.checkpoint();
    }
}

impl Drop for TestContext {
    fn drop(&mut self) {
        if std::thread::panicking() {
            unregister_test_context(self.context.ctx);
            return;
        }

        let verification = catch_unwind(AssertUnwindSafe(|| self.checkpoint()));
        unregister_test_context(self.context.ctx);
        if let Err(payload) = verification {
            resume_unwind(payload);
        }
    }
}

pub(crate) fn ffi_catch<T>(
    ctx: *mut raw::RedisModuleCtx,
    fallback: T,
    callback: impl FnOnce(&TestContextState) -> T,
) -> T {
    if !is_test_context(ctx) {
        return fallback;
    }
    let state = test_context_state(ctx);
    match catch_unwind(AssertUnwindSafe(|| callback(state))) {
        Ok(value) => value,
        Err(payload) => {
            *state.pending_panic.borrow_mut() = Some(payload);
            fallback
        }
    }
}

pub(crate) fn resume_pending_panic(ctx: *mut raw::RedisModuleCtx) {
    if !is_test_context(ctx) {
        return;
    }
    let state = test_context_state(ctx);
    let payload = state.pending_panic.borrow_mut().take();
    if let Some(payload) = payload {
        resume_unwind(payload);
    }
}

pub(super) extern "C" fn test_get_client_id(ctx: *mut raw::RedisModuleCtx) -> u64 {
    super::ffi_catch(ctx, 0, |state| {
        state.expectations.borrow().get_client_id.call(())
    })
}

pub(super) extern "C" fn test_get_client_name_by_id(
    ctx: *mut raw::RedisModuleCtx,
    client_id: u64,
) -> *mut raw::RedisModuleString {
    super::ffi_catch(ctx, null_mut(), |state| {
        match state
            .expectations
            .borrow()
            .get_client_name_by_id
            .call(client_id)
        {
            Ok(value) => value.take(),
            Err(_) => null_mut(),
        }
    })
}

pub(super) extern "C" fn test_set_client_name_by_id(
    client_id: u64,
    client_name: *mut raw::RedisModuleString,
) -> libc::c_int {
    let ctx = super::active_context();
    super::ffi_catch(ctx, raw::Status::Err as libc::c_int, |state| {
        let client_name = ValkeyString::string_as_slice(client_name).to_vec();
        state
            .expectations
            .borrow()
            .set_client_name_by_id
            .call((client_id, client_name)) as libc::c_int
    })
}

pub(super) extern "C" fn test_get_client_username_by_id(
    ctx: *mut raw::RedisModuleCtx,
    client_id: u64,
) -> *mut raw::RedisModuleString {
    super::ffi_catch(ctx, null_mut(), |state| {
        match state
            .expectations
            .borrow()
            .get_client_username_by_id
            .call(client_id)
        {
            Ok(value) => value.take(),
            Err(_) => null_mut(),
        }
    })
}

pub(super) extern "C" fn test_get_client_certificate(
    ctx: *mut raw::RedisModuleCtx,
    _client_id: u64,
) -> *mut raw::RedisModuleString {
    super::ffi_catch(ctx, null_mut(), |state| {
        match state.expectations.borrow().get_client_cert.call(()) {
            Ok(value) => value.take(),
            Err(_) => null_mut(),
        }
    })
}

pub(super) extern "C" fn test_deauthenticate_and_close_client(
    ctx: *mut raw::RedisModuleCtx,
    client_id: u64,
) -> libc::c_int {
    super::ffi_catch(ctx, raw::Status::Err as libc::c_int, |state| {
        state
            .expectations
            .borrow()
            .deauthenticate_and_close_client_by_id
            .call(client_id) as libc::c_int
    })
}

pub(super) extern "C" fn test_get_current_user_name(
    ctx: *mut raw::RedisModuleCtx,
) -> *mut raw::RedisModuleString {
    super::ffi_catch(ctx, null_mut(), |state| {
        state.expectations.borrow().get_current_user.call(()).take()
    })
}

pub(super) extern "C" fn test_create_string(
    _ctx: *mut raw::RedisModuleCtx,
    ptr: *const c_char,
    len: usize,
) -> *mut raw::RedisModuleString {
    // RedisModule_CreateString receives a C pointer plus explicit byte length;
    // rebuild the borrowed byte slice before copying it into the test string.
    let bytes = unsafe { std::slice::from_raw_parts(ptr.cast::<u8>(), len) };
    ValkeyString::test(bytes.to_vec()).take()
}

pub(super) extern "C" fn test_set_module_options(
    ctx: *mut raw::RedisModuleCtx,
    options: libc::c_int,
) {
    super::ffi_catch(ctx, (), |state| {
        state
            .expectations
            .borrow()
            .set_module_options
            .call(raw::ModuleOptions::from_bits_retain(options));
    });
}

pub(super) extern "C" fn test_authenticate_client_with_acl_user(
    ctx: *mut raw::RedisModuleCtx,
    name: *const c_char,
    len: usize,
    _callback: raw::RedisModuleUserChangedFunc,
    _privdata: *mut c_void,
    _client_id: *mut u64,
) -> libc::c_int {
    let name = unsafe { std::slice::from_raw_parts(name.cast::<u8>(), len) };
    let name = name.to_vec();
    super::ffi_catch(ctx, raw::Status::Err as libc::c_int, |state| {
        state
            .expectations
            .borrow()
            .authenticate_client_with_acl_user
            .call(name) as libc::c_int
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn get_client_id_uses_configured_expectation() {
        let mut ctx = Context::test();
        ctx.expect_get_client_id().returning(|| 42);

        assert_eq!(ctx.get_client_id(), 42);
        ctx.checkpoint();
    }

    #[test]
    fn checkpoint_allows_new_expectations() {
        let mut ctx = Context::test();
        ctx.expect_get_client_id().returning(|| 1);
        assert_eq!(ctx.get_client_id(), 1);
        ctx.checkpoint();

        ctx.expect_get_client_id().returning(|| 2);
        assert_eq!(ctx.get_client_id(), 2);
        ctx.checkpoint();
    }

    #[test]
    #[should_panic(expected = "get_client_id called without an expectation")]
    fn public_context_call_resumes_ffi_expectation_panic() {
        let ctx = Context::test();
        let _ = ctx.get_client_id();
    }

    #[test]
    #[should_panic(expected = "configured return panic")]
    fn public_context_call_resumes_return_closure_panic() {
        let mut ctx = Context::test();
        ctx.expect_get_client_id()
            .returning(|| panic!("configured return panic"));
        let _ = ctx.get_client_id();
    }

    #[test]
    fn panic_resume_ignores_unregistered_contexts() {
        resume_pending_panic(std::ptr::null_mut());
    }

    #[test]
    #[should_panic(expected = "expected 1 call(s), observed 0")]
    fn test_context_drop_verifies_expectations() {
        let mut ctx = Context::test();
        ctx.expect_get_client_id().returning(|| 42);
    }

    #[test]
    fn client_name_uses_ordered_expectations() {
        let mut ctx = Context::test();
        ctx.expect_get_client_name_by_id()
            .withf(|id| *id == 0)
            .returning(|_| Err(crate::ValkeyError::Str("missing")));
        ctx.expect_get_client_name_by_id()
            .withf(|id| *id == 42)
            .returning(|_| Ok(ValkeyString::test("client")));

        assert!(ctx.get_client_name_by_id(0).is_err());
        assert_eq!(ctx.get_client_name_by_id(42).unwrap().to_string(), "client");
        ctx.checkpoint();
    }

    #[test]
    fn set_client_name_matches_id_and_bytes() {
        let mut ctx = Context::test();
        ctx.expect_set_client_name_by_id()
            .withf(|id, name| *id == 42 && name == b"client")
            .returning(|_, _| raw::Status::Ok);

        let name = ValkeyString::test("client");
        assert_eq!(ctx.set_client_name_by_id(42, &name), raw::Status::Ok);
        ctx.checkpoint();
    }

    #[test]
    fn username_certificate_and_deauthentication_use_expectations() {
        let mut ctx = Context::test();
        ctx.expect_get_client_username_by_id()
            .withf(|id| *id == 42)
            .returning(|_| Ok(ValkeyString::test("alice")));
        ctx.expect_get_client_id().returning(|| 42);
        ctx.expect_get_client_cert()
            .returning(|| Ok(ValkeyString::test("certificate")));
        ctx.expect_deauthenticate_and_close_client_by_id()
            .withf(|id| *id == 42)
            .returning(|_| raw::Status::Ok);

        assert_eq!(
            ctx.get_client_username_by_id(42).unwrap().to_string(),
            "alice"
        );
        assert_eq!(ctx.get_client_cert().unwrap().to_string(), "certificate");
        assert_eq!(
            ctx.deauthenticate_and_close_client_by_id(42),
            raw::Status::Ok
        );
        ctx.checkpoint();
    }

    #[test]
    fn current_user_uses_expectation() {
        let mut ctx = Context::test();
        ctx.expect_get_current_user()
            .returning(|| ValkeyString::test("alice"));

        assert_eq!(ctx.get_current_user().to_string(), "alice");
        ctx.checkpoint();
    }

    #[test]
    fn module_options_are_matched() {
        let mut ctx = Context::test();
        ctx.expect_set_module_options()
            .withf(|options| options.contains(raw::ModuleOptions::HANDLE_REPL_ASYNC_LOAD))
            .returning(|_| ());

        ctx.set_module_options(raw::ModuleOptions::HANDLE_REPL_ASYNC_LOAD);
        ctx.checkpoint();
    }

    #[test]
    fn acl_authentication_uses_username_bytes() {
        let mut ctx = Context::test();
        ctx.expect_authenticate_client_with_acl_user()
            .withf(|username| username == b"alice")
            .returning(|_| raw::Status::Ok);

        let username = ValkeyString::test("alice");
        assert_eq!(
            ctx.authenticate_client_with_acl_user(&username),
            raw::Status::Ok
        );
        ctx.checkpoint();
    }

    #[test]
    fn test_context_expectations_are_isolated_between_threads() {
        let first = std::thread::spawn(|| {
            let mut ctx = Context::test();
            ctx.expect_get_client_id().returning(|| 1);
            assert_eq!(ctx.get_client_id(), 1);
            ctx.checkpoint();
        });
        let second = std::thread::spawn(|| {
            let mut ctx = Context::test();
            ctx.expect_get_client_id().returning(|| 2);
            assert_eq!(ctx.get_client_id(), 2);
            ctx.checkpoint();
        });

        first.join().expect("first test thread panicked");
        second.join().expect("second test thread panicked");
    }
}
