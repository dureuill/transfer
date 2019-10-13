use stackpin::{stack_let, FromUnpinned, PinStack, Unpinned};
use std::cell::Cell;
use std::marker::{PhantomData, PhantomPinned};
use transfer::{transfer, Tr, Transfer};

pub struct DynRef<T: ?Sized>(Cell<Option<*const T>>);

impl<T: ?Sized> DynRef<T> {
    pub fn new() -> Self {
        Self(Cell::new(None))
    }

    unsafe fn get(&self) -> Option<&T> {
        self.0.get().map(|ptr| &*ptr)
    }

    pub fn map<U, F: Fn(&T) -> U>(&self, f: F) -> Option<U> {
        unsafe { self.get().map(f) }
    }

    pub fn is_some(&self) -> bool {
        self.0.get().is_some()
    }

    pub fn is_none(&self) -> bool {
        self.0.get().is_none()
    }

    pub fn lock<'dr, 'br>(
        &'dr self,
        br: &'br T,
    ) -> Unpinned<(&'br T, &'dr Self), Lifetime<'dr, 'br, T>>
    where
        T: Sized, // FIXME: This shouldn't be required
    {
        Unpinned::new((br, self))
    }
}
struct Dropper<'dr, T: ?Sized + 'dr>(Option<&'dr DynRef<T>>);

pub struct Lifetime<'dr, 'br, T: ?Sized + 'dr + 'br> {
    dynref: Dropper<'dr, T>,
    _data: PhantomData<&'br T>,
    _pin: PhantomPinned,
}

impl<'dr, T: ?Sized + 'dr> Drop for Dropper<'dr, T> {
    fn drop(&mut self) {
        match self.0 {
            Some(DynRef(cell)) => cell.set(None),
            None => {}
        }
    }
}

impl<'dr, 'br, T> Lifetime<'dr, 'br, T> {
    fn new_empty() -> Self {
        Self {
            dynref: Dropper(None),
            _data: PhantomData,
            _pin: PhantomPinned,
        }
    }
}

unsafe impl<'dr, 'br, T> FromUnpinned<(&'br T, &'dr DynRef<T>)> for Lifetime<'dr, 'br, T> {
    type PinData = (&'br T, &'dr DynRef<T>);

    unsafe fn from_unpinned(data: Self::PinData) -> (Self, Self::PinData) {
        (Self::new_empty(), data)
    }

    unsafe fn on_pin(&mut self, (val, dynref): Self::PinData) {
        let ptr = val as *const T;
        dynref.0.set(Some(ptr));
        self.dynref = Dropper(Some(dynref));
    }
}

unsafe impl<'dr, 'br, T> Transfer for Lifetime<'dr, 'br, T> {
    fn empty() -> Tr<Self> {
        Tr::from_empty(Self::new_empty())
    }

    unsafe fn transfer(src: &mut PinStack<'_, Self>, dst: *mut Self) {
        (*dst).dynref.0 = src.dynref.0;
        src.as_mut().get_unchecked_mut().dynref.0 = None
    }
}

fn main() {
    let dr = DynRef::new();
    assert!(dr.is_none());
    {
        let s = String::from("foo");
        {
            stack_let!(_lifetime = dr.lock(&s));

            // you can throw the lifetime OK!
            std::mem::drop(_lifetime);
            assert!(dr.is_some());
        }
        assert!(dr.is_none());
    }
    println!("foo: {}", transfer_if_odd("foo"));
    println!("foobar: {}", transfer_if_odd("foobar"));
}

fn transfer_if_odd(val: &'static str) -> bool {
    let dr = DynRef::new();
    {
        let mut lifetime = Lifetime::empty();
        let s = String::from(val);
        assert!(dr.is_none());
        {
            stack_let!(inner_lifetime = dr.lock(&s));
            assert!(dr.is_some());
            if val.len() % 2 == 1 {
                transfer(inner_lifetime, &mut lifetime);
            }
            assert!(dr.is_some());
        }
        dr.is_some()
    }
}
