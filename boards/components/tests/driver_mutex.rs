//! Host tests for queued driver ownership and RAII release.

use core::cell::RefCell;
use core::mem::MaybeUninit;

use capsules_core::driver_mutex::{DriverMutex, DriverMutexAny, DriverMutexClient};
use kernel::ErrorCode;
use kernel::deferred_call::{self, DeferredCall, DeferredCallClient};
use kernel::platform::chip::ThreadIdProvider;
use kernel::utilities::cells::OptionalCell;

enum TestThread {}

// ### Safety
// This executable has one test and spawns no threads. Only its test thread
// accesses the deferred-call state throughout the executable's lifetime.
unsafe impl ThreadIdProvider for TestThread {
    fn running_thread_id() -> usize {
        0
    }
}

#[derive(Default)]
struct Client {
    guards: RefCell<Vec<DriverMutexAny>>,
}

impl DriverMutexClient for Client {
    fn ready(&'static self, resource: DriverMutexAny) {
        self.guards.borrow_mut().push(resource);
    }
}

fn new_mutex() -> &'static DriverMutex<u32> {
    let resource = Box::leak(Box::new(42));
    let clients = Box::leak(Box::new([const { OptionalCell::empty() }; 3]));
    let queue = Box::leak(Box::new([MaybeUninit::uninit(); 4]));
    let mutex = Box::leak(Box::new(DriverMutex::new(resource, clients, queue)));
    mutex.register();
    mutex
}

fn service() {
    assert!(DeferredCall::service_next_pending().is_some());
}

#[test]
fn driver_mutex_ownership() {
    deferred_call::initialize_deferred_call_state::<TestThread>();
    let mutex = new_mutex();
    let other_mutex = new_mutex();
    let first = Box::leak(Box::new(Client::default()));
    let second = Box::leak(Box::new(Client::default()));
    let third = Box::leak(Box::new(Client::default()));
    let first_client: &'static dyn DriverMutexClient = first;
    let first_handle = mutex.add_client(first_client).unwrap();
    assert!(mutex.add_client(first_client) == Some(first_handle));
    let second_handle = mutex.add_client(second).unwrap();
    let third_handle = mutex.add_client(third).unwrap();
    let extra = Box::leak(Box::new(Client::default()));
    assert!(mutex.add_client(extra).is_none());
    assert_eq!(other_mutex.request(first_handle), Err(ErrorCode::INVAL));
    assert!(!DeferredCall::has_tasks());

    assert_eq!(mutex.request(first_handle), Ok(()));
    assert_eq!(mutex.request(first_handle), Err(ErrorCode::ALREADY));
    assert_eq!(mutex.request(second_handle), Ok(()));
    assert_eq!(mutex.request(third_handle), Ok(()));
    assert!(first.guards.borrow().is_empty());
    service();
    assert_eq!(first.guards.borrow().len(), 1);
    assert!(second.guards.borrow().is_empty());
    assert!(!DeferredCall::has_tasks());
    assert_eq!(mutex.request(second_handle), Err(ErrorCode::ALREADY));
    assert_eq!(mutex.request(first_handle), Ok(()));
    assert_eq!(mutex.request(first_handle), Err(ErrorCode::ALREADY));
    assert_eq!(first.guards.borrow().len(), 1);
    service();
    assert_eq!(first.guards.borrow().len(), 2);
    drop(first.guards.borrow_mut().pop().unwrap());
    assert!(second.guards.borrow().is_empty());
    assert!(!DeferredCall::has_tasks());

    let erased = first.guards.borrow_mut().pop().unwrap();
    let typed = erased.downcast::<u32>().ok().unwrap();
    assert_eq!(*typed, 42);
    assert!(!DeferredCall::has_tasks());
    drop(typed);
    assert!(second.guards.borrow().is_empty());
    assert_eq!(mutex.request(first_handle), Ok(()));
    service();
    assert_eq!(second.guards.borrow().len(), 1);
    assert!(third.guards.borrow().is_empty());

    let erased = second.guards.borrow_mut().pop().unwrap();
    let erased = erased.downcast::<u64>().err().unwrap();
    assert!(!DeferredCall::has_tasks());
    drop(erased);
    service();
    assert_eq!(third.guards.borrow().len(), 1);
    assert!(first.guards.borrow().is_empty());
    third.guards.borrow_mut().clear();
    service();
    assert_eq!(first.guards.borrow().len(), 1);

    let other_handle = other_mutex.add_client(second).unwrap();
    assert_eq!(other_mutex.request(other_handle), Ok(()));
    service();
    assert_eq!(second.guards.borrow().len(), 1);
    first.guards.borrow_mut().clear();
    second.guards.borrow_mut().clear();
    assert!(!DeferredCall::has_tasks());

    assert_eq!(mutex.request(third_handle), Ok(()));
    service();
    third.guards.borrow_mut().clear();
    assert!(!DeferredCall::has_tasks());

    assert_eq!(mutex.request(first_handle), Ok(()));
    service();
    assert_eq!(mutex.request(second_handle), Ok(()));
    assert_eq!(mutex.request(first_handle), Ok(()));
    first.guards.borrow_mut().clear();
    assert!(second.guards.borrow().is_empty());
    assert_eq!(mutex.request(first_handle), Err(ErrorCode::ALREADY));
    service();
    assert_eq!(first.guards.borrow().len(), 1);
    assert!(second.guards.borrow().is_empty());
    assert!(!DeferredCall::has_tasks());
    first.guards.borrow_mut().clear();
    service();
    assert_eq!(second.guards.borrow().len(), 1);
    second.guards.borrow_mut().clear();
    assert!(!DeferredCall::has_tasks());
}
