#![allow(non_snake_case, non_camel_case_types, unused_must_use, dead_code)]
use libc::{c_void, c_int, c_long, c_ulong};

mod ffi {
    use libc::{c_void, c_int, c_float, c_double, c_ulong};

    #[cfg(use_double = "yes")]
    pub type ccd_real_t = c_double;

    #[cfg(not(use_double = "yes"))]
    pub type ccd_real_t = c_float;

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


extern "C" fn support_callback<F, T>(userdata: *const c_void, dir: *const ffi::ccd_vec3_t, vec: *mut ffi::ccd_vec3_t)
where
    F: FnMut(T) -> T + 'static,
    T: Into<[ffi::ccd_real_t; 3]> + From<[ffi::ccd_real_t; 3]>,
{
    let support_ptr = userdata as *mut F; // cast userdata as closure
    unsafe {
        let support = &mut (*support_ptr); // get reference to closure
        (*vec).v = support((*dir).v.into()).into(); // convert to/from generic type and call closure
        println!("vec.v: {:?}", (*vec).v);
    }
}


fn ccd_gjk_intersect<F, G, T>(support1: F, support2: G) -> bool
where
    F: FnMut(T) -> T + 'static,
    G: FnMut(T) -> T + 'static,
    T: Into<[ffi::ccd_real_t; 3]> + From<[ffi::ccd_real_t; 3]>,
{
    // move closures to heap
    let support1_data = Box::into_raw(Box::new(support1));
    let support2_data = Box::into_raw(Box::new(support2));

    // prepare state
    let mut ccd = ccd_new();
    ccd.support1 = Some(support_callback::<F, T>);
    ccd.support2 = Some(support_callback::<G, T>);
    ccd.max_iterations = 100;

    
    let result: c_int;
    unsafe {
        // call foreign function
        result = ffi::ccdGJKIntersect(support1_data as *const _, support2_data as *const _, &ccd as *const _) as i32;

        // take back the raw pointers
        Box::from_raw(support1_data as *mut F);
        Box::from_raw(support2_data as *mut F);
    }

    return result == 1;
}


#[cfg(test)]
mod tests {

    use crate::ccd_gjk_intersect;
    use glam::DVec3;

    #[test]
    fn pls_no_crash() {

        // closure for sphere 1
        let sphere_support_1 = |dir: DVec3| -> DVec3 {

            let dir = dir.normalize();
            let origin = DVec3::new(1.0, 0.0, 0.0);
            let radius = 2.0;

            return origin + dir * radius;
        };

        // closure for sphere 2
        let sphere_support_2 = |dir: DVec3| -> DVec3 {

            let dir = dir.normalize();
            let origin = DVec3::new(-1.0, 0.0, 0.0);
            let radius = 2.0;

            return origin + dir * radius;
        };

        let result = ccd_gjk_intersect(sphere_support_1, sphere_support_2);

        assert_eq!(result, true);
    }
}