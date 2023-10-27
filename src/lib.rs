
/*

rust wrapper for libccd

intersection functions take userdata pointers (representing the colliding instances) and a ccd_t struct which provides state and configurations

-we want to define support functions in rust
-we cant call c functions directly due to their inherently unsafe design
-ccd_t can probably be created and initialized in rust
-userdata pointers are only passed on to be dereferenced in our own (rust) functions

- we need a glue function that takes pure rust support functions and configures ccd_t accordingly



ccd_vec3_t is an array of 3 ccd_real_t (aka doubles or floats)

this means that our rust api could possibly look like:
    
    fn ccd_gjk_intersect<T>(support1: fn([T; 3]) -> [T; 3], support2: fn([T; 3]) -> [T; 3]) -> IntersectResult

    alternatively:

    fn ccd_gjk_intersect<T: Into<[f64; 3]>>(support1: fn(T) -> T, support2: fn(T) -> T) -> IntersectResult

    or:

    fn ccd_gjk_intersect<T: Into<[f64; 3]>, F: Fn(T) -> T>(support1: F, support2: F) -> IntersectResult


use fn prepare_ccd_t


TODO:
    -implement ccd_t in accordance with the c-spec
        -requires ccd_first_dir_fn type => function pointer that takes two userdata pointers and a vec3 pointer to write results
        -requires ccd_support_fn type   => function pointer that takes a userdata pointer, a direction vector pointer, and a vec3 pointer to write results
        -requires ccd_center_fn         => function pointer that takes a userdata pointer and a vec3 pointer to write results
        -requires unsigned long
        -requires ccd_real_t type       => chosen to be float or double when building libccd.a

*/

#![allow(non_snake_case, non_camel_case_types, unused_must_use, dead_code)]
use libc::{c_void, c_int, c_long, c_ulong};

mod ffi {
    use libc::{c_void, c_int, c_double, c_ulong};

    pub type ccd_real_t = c_double; // IMPORTANT: libccd.a uses ccd_real_t <=> float OR double depending on compile flags (double is default)

    #[repr(C)]
    #[derive(Copy, Clone)]
    pub struct ccd_vec3_t {
        pub v: [ccd_real_t; 3],
    }

    pub type ccd_first_dir_fn = unsafe extern "C" fn(obj1: *const c_void, obj1: *const c_void, dir: *mut ccd_vec3_t);
    pub type ccd_support_fn = unsafe extern "C" fn(obj: *const c_void, dir: *const ccd_vec3_t, vec: *mut ccd_vec3_t);
    pub type ccd_center_fn = unsafe extern "C" fn(obj: *const c_void, center: *mut ccd_vec3_t);

    #[repr(C)]
    pub struct ccd_t {
        pub first_dir: ccd_first_dir_fn,
        
        pub support1: Option<ccd_support_fn>,
        pub support2: Option<ccd_support_fn>,

        pub center1: Option<ccd_center_fn>,
        pub center2: Option<ccd_center_fn>,

        pub max_iterations: c_ulong,

        pub epa_tolerance: ccd_real_t,
        pub mpr_tolerance: ccd_real_t,
        pub dist_tolerance: ccd_real_t,
    }

    //#[link(name = "ccd", kind = "static")]
    extern "C" {
        pub fn ccdFirstDirDefault(o1: *const c_void, o2: *const c_void, dir: *mut ccd_vec3_t);
        pub fn ccdGJKIntersect(obj1: *const c_void, obj2: *const c_void, ccd: *const ccd_t) -> c_int;
    }
}


fn ccd_new() -> ffi::ccd_t {
    return ffi::ccd_t {
        first_dir: ffi::ccdFirstDirDefault,
        support1: None,
        support2: None,
        center1: None,
        center2: None,
       
        max_iterations: unsafe { std::mem::transmute::<c_long, c_ulong>(-1) }, // long and ulong should have same width, max value is expected behaviour
        epa_tolerance: 0.0001 as ffi::ccd_real_t,
        mpr_tolerance: 0.0001 as ffi::ccd_real_t,
        dist_tolerance: 1e-6 as ffi::ccd_real_t,
    };
}


extern "C" fn support_callback<F>(userdata: *const c_void, dir: *const ffi::ccd_vec3_t, vec: *mut ffi::ccd_vec3_t)
where
    F: FnMut(ffi::ccd_vec3_t) -> ffi::ccd_vec3_t + 'static,
{
    let support_ptr = userdata as *mut F; // cast userdata as closure
    unsafe {
        let support = &mut (*support_ptr); // get reference to closure
        *vec = support(*dir); // call closure
        println!("vec.v: {:?}", (*vec).v);
    }
}


fn ccd_gjk_intersect<F, G>(support1: F, support2: G) -> bool
where
    F: FnMut(ffi::ccd_vec3_t) -> ffi::ccd_vec3_t + 'static,
    G: FnMut(ffi::ccd_vec3_t) -> ffi::ccd_vec3_t + 'static,
{
    // move closures to heap
    let support1_data = Box::into_raw(Box::new(support1));
    let support2_data = Box::into_raw(Box::new(support2));

    // prepare state
    let mut ccd = ccd_new();
    ccd.support1 = Some(support_callback::<F>);
    ccd.support2 = Some(support_callback::<G>);
    ccd.max_iterations = 100;

    // call foreign function
    let result: c_int;
    unsafe{ result = ffi::ccdGJKIntersect(support1_data as *const _, support2_data as *const _, &ccd as *const _) as i32; }
    
    // take back the raw pointers
    unsafe {
        Box::from_raw(support1_data as *mut F);
        Box::from_raw(support2_data as *mut F);
    }

    return result == 1;
}


#[cfg(test)]
mod tests {

    use crate::ffi;
    use crate::ccd_gjk_intersect;

    #[test]
    fn pls_no_crash() {

        // closure for sphere 1
        let sphere_support_1 = |dir: ffi::ccd_vec3_t| -> ffi::ccd_vec3_t {

            let len = (dir.v[0]*dir.v[0] + dir.v[1]*dir.v[1] + dir.v[2]*dir.v[2]).sqrt();
            let (dx, dy, dz) = (dir.v[0] / len, dir.v[1] / len, dir.v[2] / len);

            let (ox, oy, oz) = (1.0, 0.0, 0.0);
            let r = 2.0;

            return ffi::ccd_vec3_t { v: [ox + dx * r, oy + dy * r, oz + dz * r] };
        };

        // closure for sphere 2
        let sphere_support_2 = |dir: ffi::ccd_vec3_t| -> ffi::ccd_vec3_t {
            
            let len = (dir.v[0]*dir.v[0] + dir.v[1]*dir.v[1] + dir.v[2]*dir.v[2]).sqrt();
            let (dx, dy, dz) = (dir.v[0] / len, dir.v[1] / len, dir.v[2] / len);
            
            let (ox, oy, oz) = (-1.0, 0.0, 0.0);
            let r = 2.0;

            return ffi::ccd_vec3_t { v: [ox + dx * r, oy + dy * r, oz + dz * r] };
        };

        let result = ccd_gjk_intersect(sphere_support_1, sphere_support_2);

        assert_eq!(result, true);
    }
}