use stackpin::PinStack;

pub trait Transfer: Sized {
    unsafe fn transfer(src: &PinStack<'_, Self>, dst: *mut Self) {
        use std::ptr;
        let src = ptr::read(src.as_ref().get_ref());
        ptr::write(dst, src);
    }

    unsafe fn set_empty(pin: &mut PinStack<'_, Self>);

    fn empty() -> Self;
}

/// Slot<T> is "just a T you can't use". It is guaranteed to have the same layout than T, but none of the T members
/// are available. Slot is used internally by the `transfer` machinery to write an actual T in its place.
/// Since the Slot<T> contains a T, it **will** call T's destructor on drop. 
#[repr(transparent)]
pub struct Slot<T>(T);

impl<T: Transfer> Slot<T> {
    pub fn empty() -> Self {
        Self(T::empty())
    }

    fn as_ptr(&mut self) -> *mut T {
        &mut self.0 as *mut T
    }
}

pub fn transfer<'old, 'new, T: Transfer>(
    mut src: PinStack<'old, T>,
    dest: &'new mut Slot<T>,
) -> PinStack<'new, T> {
    use stackpin::StackPinned;
    use std::pin::Pin;
    unsafe {
        <T as Transfer>::transfer(&src, dest.as_ptr());
        <T as Transfer>::set_empty(&mut src);
        Pin::new_unchecked(StackPinned::new(&mut dest.0))
    }
}

#[macro_export]
macro_rules! transfer_let {
    ($id:ident = $fun_name:ident ($($arg:expr),*)) => {
        let mut $id = $crate::Slot::empty();
        let $id = $fun_name($($arg),* &mut $id);
    };
    /*($id:ident = $e:expr) => {
        let $id = $crate::Slot::empty();
        $crate::transfer($e, &mut $id);
    };*/
}

#[cfg(test)]
mod tests {

    mod secret {
        use std::marker::PhantomPinned;
        pub struct SecretU64(u64, PhantomPinned);

        fn secure_erase(x: &mut u64) {
            *x = 0;
        }

        use super::super::Empty;
        use super::super::Transfer;
        use stackpin::FromUnpinned;
        use stackpin::PinStack;

        impl<'a> FromUnpinned<&'a mut u64> for SecretU64 {
            type PinData = &'a mut u64;

            unsafe fn from_unpinned(src: &'a mut u64) -> (Self, &'a mut u64) {
                (Self(0, PhantomPinned), src)
            }

            unsafe fn on_pin(&mut self, data: &'a mut u64) {
                self.0 = *data;
                println!(
                    "Secure erasing data that served for construction at {:p}",
                    data
                );
                secure_erase(data);
            }
        }

        impl Transfer for SecretU64 {
            unsafe fn set_empty(pin: &mut PinStack<'_, Self>) {
                println!(
                    "Secure erasing on transfer for {:p}",
                    &mut pin.as_mut().get_unchecked_mut().0
                );
                secure_erase(&mut pin.as_mut().get_unchecked_mut().0)
            }

            fn empty() -> Self {
                Self(0, PhantomPinned)
            }
        }

        impl SecretU64 {
            pub fn reveal(this: &PinStack<'_, Self>) -> u64 {
                this.0
            }
        }

        impl Drop for SecretU64 {
            fn drop(&mut self) {
                if self.0 == 0 {
                    println!("Not erasing empty secret at {:p}", self);
                } else {
                    println!("Secure erasing in dtor for {:p}", self);
                    secure_erase(&mut self.0)
                }
            }
        }

        pub fn generate_secret(slot: &mut crate::Slot<SecretU64>) -> PinStack<'_, SecretU64> {
            let mut secret = 42;
            stackpin::stack_let!(secret = stackpin::Unpinned::new(&mut secret));
            crate::transfer(secret, slot)
        }
    }

    #[test]
    fn it_works() {
        use secret::generate_secret;
        super::transfer_let!(my_secret = generate_secret());
        println!(
            "Revealing value of my secret: {}",
            secret::SecretU64::reveal(&my_secret)
        );
    }
}
