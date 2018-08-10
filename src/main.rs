
extern crate weld;
extern crate time;
extern crate rand;
extern crate fnv;

use std::mem;
use std::cell::Cell;

use weld::*;
use time::*;
use fnv::FnvHashMap;

use rand::{Rng, FromEntropy};
use rand::rngs::SmallRng;

#[repr(C)]
#[derive(Clone)]
pub struct WeldVec<T> {
    data: *mut T,
    size: i64,
}

#[repr(C)]
#[derive(Clone)]
pub struct Pair<T, U> {
    e1: T,
    e2: U,
}

impl<T> WeldVec<T> {
    pub fn new(data: *mut T, size: i64) -> Self {
        WeldVec {
            data: data,
            size: size
        }
    }
}

fn input_data(size: usize, groups: i32) -> Vec<i32> {
    let elements = size / mem::size_of::<i32>();
    println!("elements: {} groups: {}", elements, groups);
    let mut vector = Vec::with_capacity(elements);

    let mut rng = SmallRng::from_entropy();
    
    // Random data that is uniformly distributed.
    for _ in 0..elements {
        vector.push(rng.gen_range(0, groups));
    }
    vector
}

fn main() {
    // Creates self-groups of vectors.
    let program = "|v: vec[i32]|
            let d = result(for(v,
                groupmerger[i32,i32],
                |b,i,e|
                    merge(b, {e, e})
            ));
            tovec(d)";

    let ref mut conf = WeldConf::new();

    // conf.set("weld.compile.backend", "workstealing");

    let start = PreciseTime::now();
    let mut module = WeldModule::compile(program, conf).unwrap();
    let end = PreciseTime::now();
    println!("Compile time: {} ms", start.to(end).num_milliseconds());

    let groups = 10;

    // 2GB of data.
    let mut data = input_data(2_000_000_000, groups);
    println!("Finished generating input data.");

    let start = PreciseTime::now();
    let mut map = FnvHashMap::default();
    for i in data.iter() {
        let mut vector = map.entry(*i).or_insert(Vec::new());
        vector.push(*i);
    }
    let mut kv_pairs: Vec<(i32, (*const i32, usize))> = Vec::with_capacity(map.len());
    for (key, value) in map.iter() {
        kv_pairs.push((*key, (value.as_ptr(), value.len())));
    }
    let end = PreciseTime::now();
    println!("Rust Runtime: {} ms", start.to(end).num_milliseconds());

    for val in kv_pairs.iter() {
        println!("{:?}", val);
    }

    let ref mut conf = WeldConf::new();
    
    // 8GB memory limit.
    conf.set("weld.memory.limit", "8000000000");

    let weld_input = Cell::new(WeldVec::new(data.as_mut_ptr(), data.len() as i64));
    let ref value = WeldValue::new_from_data(weld_input.as_ptr() as Data);

    let start = PreciseTime::now();
    let result = unsafe {module.run(conf, value).unwrap() };
    let end = PreciseTime::now();
    println!("Runtime: {} ms", start.to(end).num_milliseconds());

    let data: *const WeldVec<Pair<i32, WeldVec<i32>>> = result.data() as _;
    let r = unsafe { (*data).clone() };

    let length = r.size;

    assert_eq!(length as i32, groups);
    let mut compare_result = Vec::with_capacity(groups as usize);
    for i in 0..(length as isize) {
        let key = unsafe { (*r.data.offset(i)).e1 };
        let value = unsafe { (*r.data.offset(i)).e2.clone() };
        println!("({}, ({:?}, {})", key, value.data, value.size);
        compare_result.push((key, value.size));
    }

    let mut kv_pairs_no_ptr = kv_pairs
        .into_iter()
        .map(|(key, (_, sz))| (key as i32, sz as i64))
        .collect::<Vec<_>>();

    compare_result.sort();
    kv_pairs_no_ptr.sort();

    assert_eq!(compare_result, kv_pairs_no_ptr);
}
