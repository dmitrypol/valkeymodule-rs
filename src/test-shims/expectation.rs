use std::cell::RefCell;
use std::fmt::Debug;
use std::marker::PhantomData;
use std::rc::Rc;

type Matcher<Args> = Box<dyn Fn(&Args) -> bool>;
type Returner<Args, Output> = Box<dyn FnMut(Args) -> Output>;

pub(crate) struct ExpectedCall<Args, Output> {
    matcher: Matcher<Args>,
    returner: Option<Returner<Args, Output>>,
    expected_calls: usize,
    actual_calls: usize,
}

pub(crate) struct MethodMock<Args, Output> {
    method_name: &'static str,
    calls: Vec<Rc<RefCell<ExpectedCall<Args, Output>>>>,
}

impl<Args, Output> MethodMock<Args, Output>
where
    Args: Debug,
{
    pub(crate) fn new(method_name: &'static str) -> Self {
        Self {
            method_name,
            calls: Vec::new(),
        }
    }

    pub(crate) fn expect(&mut self) -> Rc<RefCell<ExpectedCall<Args, Output>>> {
        let expectation = Rc::new(RefCell::new(ExpectedCall {
            matcher: Box::new(|_| true),
            returner: None,
            expected_calls: 1,
            actual_calls: 0,
        }));
        self.calls.push(Rc::clone(&expectation));
        expectation
    }

    pub(crate) fn call(&self, args: Args) -> Output {
        if self.calls.is_empty() {
            panic!(
                "{} called without an expectation; args: {args:?}",
                self.method_name
            );
        }

        let mut matched = false;
        let mut selected = None;
        for candidate in &self.calls {
            let expectation = candidate.borrow();
            if !(expectation.matcher)(&args) {
                continue;
            }
            matched = true;
            if expectation.actual_calls < expectation.expected_calls {
                selected = Some(Rc::clone(candidate));
                break;
            }
        }

        let Some(selected) = selected else {
            if matched {
                panic!(
                    "{} called more times than expected; args: {args:?}",
                    self.method_name
                );
            }
            panic!(
                "{} called with unmatched arguments: {args:?}",
                self.method_name
            );
        };

        let mut expectation = selected.borrow_mut();
        expectation.actual_calls += 1;
        let Some(returner) = expectation.returner.as_mut() else {
            panic!("{} expectation has no returning closure", self.method_name);
        };
        returner(args)
    }

    pub(crate) fn checkpoint(&mut self) {
        for expectation in &self.calls {
            let expectation = expectation.borrow();
            if expectation.actual_calls != expectation.expected_calls {
                panic!(
                    "{} expected {} call(s), observed {}",
                    self.method_name, expectation.expected_calls, expectation.actual_calls
                );
            }
        }
        self.calls.clear();
    }
}

pub struct Expectation0<'a, Output> {
    inner: Rc<RefCell<ExpectedCall<(), Output>>>,
    context_borrow: PhantomData<&'a mut ()>,
}

impl<'a, Output> Expectation0<'a, Output> {
    pub(crate) fn new(inner: Rc<RefCell<ExpectedCall<(), Output>>>) -> Self {
        Self {
            inner,
            context_borrow: PhantomData,
        }
    }

    pub fn times(&mut self, count: usize) -> &mut Self {
        self.inner.borrow_mut().expected_calls = count;
        self
    }

    pub fn returning<F>(&mut self, mut callback: F) -> &mut Self
    where
        F: FnMut() -> Output + 'static,
    {
        self.inner.borrow_mut().returner = Some(Box::new(move |()| callback()));
        self
    }
}

pub struct Expectation1<'a, Arg, Output> {
    inner: Rc<RefCell<ExpectedCall<Arg, Output>>>,
    context_borrow: PhantomData<&'a mut ()>,
}

impl<'a, Arg, Output> Expectation1<'a, Arg, Output> {
    pub(crate) fn new(inner: Rc<RefCell<ExpectedCall<Arg, Output>>>) -> Self {
        Self {
            inner,
            context_borrow: PhantomData,
        }
    }

    pub fn withf<F>(&mut self, matcher: F) -> &mut Self
    where
        F: Fn(&Arg) -> bool + 'static,
    {
        self.inner.borrow_mut().matcher = Box::new(matcher);
        self
    }

    pub fn times(&mut self, count: usize) -> &mut Self {
        self.inner.borrow_mut().expected_calls = count;
        self
    }

    pub fn returning<F>(&mut self, callback: F) -> &mut Self
    where
        F: FnMut(Arg) -> Output + 'static,
    {
        self.inner.borrow_mut().returner = Some(Box::new(callback));
        self
    }
}

pub struct Expectation2<'a, Arg1, Arg2, Output> {
    inner: Rc<RefCell<ExpectedCall<(Arg1, Arg2), Output>>>,
    context_borrow: PhantomData<&'a mut ()>,
}

impl<'a, Arg1, Arg2, Output> Expectation2<'a, Arg1, Arg2, Output> {
    pub(crate) fn new(inner: Rc<RefCell<ExpectedCall<(Arg1, Arg2), Output>>>) -> Self {
        Self {
            inner,
            context_borrow: PhantomData,
        }
    }

    pub fn withf<F>(&mut self, matcher: F) -> &mut Self
    where
        F: Fn(&Arg1, &Arg2) -> bool + 'static,
    {
        self.inner.borrow_mut().matcher = Box::new(move |(arg1, arg2)| matcher(arg1, arg2));
        self
    }

    pub fn times(&mut self, count: usize) -> &mut Self {
        self.inner.borrow_mut().expected_calls = count;
        self
    }

    pub fn returning<F>(&mut self, mut callback: F) -> &mut Self
    where
        F: FnMut(Arg1, Arg2) -> Output + 'static,
    {
        self.inner.borrow_mut().returner = Some(Box::new(move |(arg1, arg2)| callback(arg1, arg2)));
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expectation_zero_returns_configured_value() {
        let mut method = MethodMock::<(), u64>::new("get_client_id");
        Expectation0::new(method.expect()).returning(|| 42);

        assert_eq!(method.call(()), 42);
        method.checkpoint();
    }

    #[test]
    fn expectation_zero_honors_times() {
        let mut method = MethodMock::<(), u64>::new("get_client_id");
        Expectation0::new(method.expect()).times(2).returning(|| 42);

        assert_eq!(method.call(()), 42);
        assert_eq!(method.call(()), 42);
        method.checkpoint();
    }

    #[test]
    fn expectation_one_selects_first_matching_available_expectation() {
        let mut method = MethodMock::<u64, &'static str>::new("get_name");
        Expectation1::new(method.expect())
            .withf(|id| *id == 0)
            .returning(|_| "missing");
        Expectation1::new(method.expect())
            .withf(|id| *id == 42)
            .returning(|_| "client");

        assert_eq!(method.call(0), "missing");
        assert_eq!(method.call(42), "client");
        method.checkpoint();
    }

    #[test]
    fn expectation_two_passes_both_arguments_to_matcher_and_returner() {
        let mut method = MethodMock::<(u64, Vec<u8>), bool>::new("set_name");
        Expectation2::new(method.expect())
            .withf(|id, name| *id == 42 && name == b"client")
            .returning(|id, name| id == 42 && name == b"client");

        assert!(method.call((42, b"client".to_vec())));
        method.checkpoint();
    }

    #[test]
    #[should_panic(expected = "called without an expectation")]
    fn unconfigured_method_panics() {
        MethodMock::<(), u64>::new("get_client_id").call(());
    }

    #[test]
    #[should_panic(expected = "called with unmatched arguments")]
    fn unmatched_arguments_panic() {
        let mut method = MethodMock::<u64, u64>::new("get_name");
        Expectation1::new(method.expect())
            .withf(|id| *id == 42)
            .returning(|id| id);
        method.call(7);
    }

    #[test]
    #[should_panic(expected = "called more times than expected")]
    fn excess_calls_panic() {
        let mut method = MethodMock::<(), u64>::new("get_client_id");
        Expectation0::new(method.expect()).returning(|| 42);
        method.call(());
        method.call(());
    }

    #[test]
    #[should_panic(expected = "expectation has no returning closure")]
    fn missing_returning_panics() {
        let mut method = MethodMock::<(), u64>::new("get_client_id");
        method.expect();
        method.call(());
    }

    #[test]
    #[should_panic(expected = "expected 1 call(s), observed 0")]
    fn unmet_count_panics_at_checkpoint() {
        let mut method = MethodMock::<(), u64>::new("get_client_id");
        Expectation0::new(method.expect()).returning(|| 42);
        method.checkpoint();
    }
}
