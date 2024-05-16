#![feature(strict_provenance, pointer_is_aligned_to)]
use std::{mem, ptr, slice};

fn test_memcpy() {
    unsafe {
        let src = [1i8, 2, 3];
        let dest = libc::calloc(3, 1);
        libc::memcpy(dest, src.as_ptr() as *const libc::c_void, 3);
        let slc = std::slice::from_raw_parts(dest as *const i8, 3);
        assert_eq!(*slc, [1i8, 2, 3]);
        libc::free(dest);
    }

    unsafe {
        let src = [1i8, 2, 3];
        let dest = libc::calloc(4, 1);
        libc::memcpy(dest, src.as_ptr() as *const libc::c_void, 3);
        let slc = std::slice::from_raw_parts(dest as *const i8, 4);
        assert_eq!(*slc, [1i8, 2, 3, 0]);
        libc::free(dest);
    }

    unsafe {
        let src = 123_i32;
        let mut dest = 0_i32;
        libc::memcpy(
            &mut dest as *mut i32 as *mut libc::c_void,
            &src as *const i32 as *const libc::c_void,
            mem::size_of::<i32>(),
        );
        assert_eq!(dest, src);
    }

    unsafe {
        let src = Some(123);
        let mut dest: Option<i32> = None;
        libc::memcpy(
            &mut dest as *mut Option<i32> as *mut libc::c_void,
            &src as *const Option<i32> as *const libc::c_void,
            mem::size_of::<Option<i32>>(),
        );
        assert_eq!(dest, src);
    }

    unsafe {
        let src = &123;
        let mut dest = &42;
        libc::memcpy(
            &mut dest as *mut &'static i32 as *mut libc::c_void,
            &src as *const &'static i32 as *const libc::c_void,
            mem::size_of::<&'static i32>(),
        );
        assert_eq!(*dest, 123);
    }
}

fn test_strcpy() {
    use std::ffi::{CStr, CString};

    // case: src_size equals dest_size
    unsafe {
        let src = CString::new("rust").unwrap();
        let size = src.as_bytes_with_nul().len();
        let dest = libc::malloc(size);
        libc::strcpy(dest as *mut libc::c_char, src.as_ptr());
        assert_eq!(CStr::from_ptr(dest as *const libc::c_char), src.as_ref());
        libc::free(dest);
    }

    // case: src_size is less than dest_size
    unsafe {
        let src = CString::new("rust").unwrap();
        let size = src.as_bytes_with_nul().len();
        let dest = libc::malloc(size + 1);
        libc::strcpy(dest as *mut libc::c_char, src.as_ptr());
        assert_eq!(CStr::from_ptr(dest as *const libc::c_char), src.as_ref());
        libc::free(dest);
    }
}

fn test_malloc() {
    // Test that small allocations sometimes *are* not very aligned.
    let saw_unaligned = (0..64).any(|_| unsafe {
        let p = libc::malloc(3);
        libc::free(p);
        (p as usize) % 4 != 0 // find any that this is *not* 4-aligned
    });
    assert!(saw_unaligned);

    unsafe {
        let p1 = libc::malloc(20);
        p1.write_bytes(0u8, 20);

        // old size < new size
        let p2 = libc::realloc(p1, 40);
        let slice = slice::from_raw_parts(p2 as *const u8, 20);
        assert_eq!(&slice, &[0_u8; 20]);

        // old size == new size
        let p3 = libc::realloc(p2, 40);
        let slice = slice::from_raw_parts(p3 as *const u8, 20);
        assert_eq!(&slice, &[0_u8; 20]);

        // old size > new size
        let p4 = libc::realloc(p3, 10);
        let slice = slice::from_raw_parts(p4 as *const u8, 10);
        assert_eq!(&slice, &[0_u8; 10]);

        libc::free(p4);
    }

    unsafe {
        // Realloc with size 0 is okay for the null pointer (and acts like `malloc(0)`)
        let p2 = libc::realloc(ptr::null_mut(), 0);
        assert!(!p2.is_null());
        libc::free(p2);
    }

    unsafe {
        let p1 = libc::realloc(ptr::null_mut(), 20);
        assert!(!p1.is_null());

        libc::free(p1);
    }
}

fn test_calloc() {
    unsafe {
        let p1 = libc::calloc(0, 0);
        assert!(!p1.is_null());
        libc::free(p1);

        let p2 = libc::calloc(20, 0);
        assert!(!p2.is_null());
        libc::free(p2);

        let p3 = libc::calloc(0, 20);
        assert!(!p3.is_null());
        libc::free(p3);

        let p4 = libc::calloc(4, 8);
        assert!(!p4.is_null());
        let slice = slice::from_raw_parts(p4 as *const u8, 4 * 8);
        assert_eq!(&slice, &[0_u8; 4 * 8]);
        libc::free(p4);
    }
}

#[cfg(not(target_os = "windows"))]
fn test_memalign() {
    // A normal allocation.
    unsafe {
        let mut ptr: *mut libc::c_void = ptr::null_mut();
        let align = 8;
        let size = 64;
        assert_eq!(libc::posix_memalign(&mut ptr, align, size), 0);
        assert!(!ptr.is_null());
        assert!(ptr.is_aligned_to(align));
        ptr.cast::<u8>().write_bytes(1, size);
        libc::free(ptr);
    }

    // Align > size.
    unsafe {
        let mut ptr: *mut libc::c_void = ptr::null_mut();
        let align = 64;
        let size = 8;
        assert_eq!(libc::posix_memalign(&mut ptr, align, size), 0);
        assert!(!ptr.is_null());
        assert!(ptr.is_aligned_to(align));
        ptr.cast::<u8>().write_bytes(1, size);
        libc::free(ptr);
    }

    // Size not multiple of align
    unsafe {
        let mut ptr: *mut libc::c_void = ptr::null_mut();
        let align = 16;
        let size = 31;
        assert_eq!(libc::posix_memalign(&mut ptr, align, size), 0);
        assert!(!ptr.is_null());
        assert!(ptr.is_aligned_to(align));
        ptr.cast::<u8>().write_bytes(1, size);
        libc::free(ptr);
    }

    // Size == 0
    unsafe {
        let mut ptr: *mut libc::c_void = ptr::null_mut();
        let align = 64;
        let size = 0;
        assert_eq!(libc::posix_memalign(&mut ptr, align, size), 0);
        // Non-null pointer is returned if size == 0.
        // (This is not a guarantee, it just reflects our current behavior.)
        assert!(!ptr.is_null());
        assert!(ptr.is_aligned_to(align));
        libc::free(ptr);
    }

    // Non-power of 2 align
    unsafe {
        let mut ptr: *mut libc::c_void = ptr::without_provenance_mut(0x1234567);
        let align = 15;
        let size = 8;
        assert_eq!(libc::posix_memalign(&mut ptr, align, size), libc::EINVAL);
        // The pointer is not modified on failure, posix_memalign(3) says:
        // > On Linux (and other systems), posix_memalign() does  not  modify  memptr  on failure.
        // > A requirement standardizing this behavior was added in POSIX.1-2008 TC2.
        assert_eq!(ptr.addr(), 0x1234567);
    }

    // Too small align (smaller than ptr)
    unsafe {
        let mut ptr: *mut libc::c_void = ptr::without_provenance_mut(0x1234567);
        let align = std::mem::size_of::<usize>() / 2;
        let size = 8;
        assert_eq!(libc::posix_memalign(&mut ptr, align, size), libc::EINVAL);
        // The pointer is not modified on failure, posix_memalign(3) says:
        // > On Linux (and other systems), posix_memalign() does  not  modify  memptr  on failure.
        // > A requirement standardizing this behavior was added in POSIX.1-2008 TC2.
        assert_eq!(ptr.addr(), 0x1234567);
    }
}

#[cfg(not(any(
    target_os = "windows",
    target_os = "macos",
    target_os = "illumos",
    target_os = "solaris",
    target_os = "wasi",
)))]
fn test_reallocarray() {
    unsafe {
        let mut p = libc::reallocarray(std::ptr::null_mut(), 4096, 2);
        assert!(!p.is_null());
        libc::free(p);
        p = libc::malloc(16);
        let r = libc::reallocarray(p, 2, 32);
        assert!(!r.is_null());
        libc::free(r);
    }
}

fn main() {
    test_malloc();
    test_calloc();
    #[cfg(not(target_os = "windows"))]
    test_memalign();
    #[cfg(not(any(
        target_os = "windows",
        target_os = "macos",
        target_os = "illumos",
        target_os = "solaris",
        target_os = "wasi",
    )))]
    test_reallocarray();

    test_memcpy();
    test_strcpy();
}
