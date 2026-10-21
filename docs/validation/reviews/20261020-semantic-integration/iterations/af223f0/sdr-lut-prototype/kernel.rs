
use std::{hint::black_box,time::Instant};
fn srgb_decode(value: f32) -> f32 {
    if value <= 0.040_45 {
        value / 12.92
    } else {
        ((value + 0.055) / 1.055).powf(2.4)
    }
}
fn baseline(input:&[u8])->Vec<f32>{input.iter().map(|&c|srgb_decode(f32::from(c)/255.0)).collect()}
fn lookup(input:&[u8])->Vec<f32>{let table:[f32;256]=std::array::from_fn(|i|srgb_decode(i as f32/255.0));input.iter().map(|&c|table[usize::from(c)]).collect()}
fn main(){
 let input=std::fs::read(std::env::args().nth(1).unwrap()).unwrap();
 let all:Vec<u8>=(0..=255).collect();let a=baseline(&all);let b=lookup(&all);
 assert!(a.iter().zip(&b).all(|(a,b)|a.to_bits()==b.to_bits()));
 let a=baseline(&input);let b=lookup(&input);assert!(a.iter().zip(&b).all(|(a,b)|a.to_bits()==b.to_bits()));drop((a,b));
 black_box(baseline(black_box(&input)));black_box(lookup(black_box(&input)));
 let mut rows=vec![];
 for iteration in 0..8 {
  for arm in if iteration%2==0 {["baseline","lookup"]}else{["lookup","baseline"]} {
   let now=Instant::now();let output=if arm=="baseline"{baseline(black_box(&input))}else{lookup(black_box(&input))};black_box(&output);
   rows.push(format!("{{\"iteration\":{},\"arm\":\"{}\",\"seconds\":{}}}",iteration,arm,now.elapsed().as_secs_f64()));
  }
 }
 println!("{{\"all_256_bit_differences\":0,\"real_component_count\":{},\"real_bit_differences\":0,\"samples\":[{}]}}",input.len(),rows.join(","));
}
