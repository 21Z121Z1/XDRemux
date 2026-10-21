use std::{fs, hint::black_box, io::{BufWriter, Write}, time::Instant};
use xdremux_hdr::{apply_iso_gain_sample, reconstruct_scene_linear_hdr, GainMapCodes, HdrError, IsoGainApplication, IsoGainChannel, SceneLinearRgb};

fn channel() -> IsoGainChannel {
    IsoGainChannel {minimum_log2:-0.25, maximum_log2:3.5, gamma:1.0, base_offset:1.0/64.0, alternate_offset:1.0/128.0}
}
fn application() -> IsoGainApplication {IsoGainApplication {channels:[channel();3],weight:1.0}}
fn u64out(out:&mut impl Write,v:u64) {out.write_all(&v.to_le_bytes()).unwrap();}
fn error(out:&mut impl Write,e:&HdrError) {
    let text=format!("{e}\n{e:?}");u64out(out,text.len() as u64);out.write_all(text.as_bytes()).unwrap();
}
fn record(out:&mut impl Write,name:&str,result:Result<SceneLinearRgb,HdrError>) -> bool {
    u64out(out,name.len() as u64);out.write_all(name.as_bytes()).unwrap();
    match result {
        Ok(image)=>{out.write_all(b"O").unwrap();u64out(out,image.width as u64);u64out(out,image.height as u64);u64out(out,image.samples.len() as u64);for v in image.samples {out.write_all(&v.to_bits().to_le_bytes()).unwrap();}true},
        Err(e)=>{out.write_all(b"E").unwrap();error(out,&e);false}
    }
}
fn scalar_record(out:&mut impl Write,sdr:f64,code:f64,c:&IsoGainChannel,weight:f64)->bool {
    match apply_iso_gain_sample(sdr,code,c,weight) {
        Ok(v)=>{out.write_all(b"O").unwrap();u64out(out,v.to_bits());true},
        Err(e)=>{out.write_all(b"E").unwrap();error(out,&e);false}
    }
}
fn next(seed:&mut u64)->u64 {*seed=seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);*seed}
fn unit(seed:&mut u64)->f64 {(next(seed)>>11) as f64 / ((1u64<<53) as f64)}
fn matrix(path:&str) {
    let mut out=BufWriter::new(fs::File::create(path).unwrap());out.write_all(b"HDRMATRX").unwrap();
    let mut reconstructions=0;let mut reconstruction_ok=0;let mut reconstruction_err=0;
    let mut run=|name:&str,sdr:&SceneLinearRgb,gain:GainMapCodes<'_>,app:&IsoGainApplication| {
        reconstructions+=1;if record(&mut out,name,reconstruct_scene_linear_hdr(sdr,gain,app)){reconstruction_ok+=1;}else{reconstruction_err+=1;}
    };
    let finite=[0.0,-0.0,1.0,-1.0,f32::MIN_POSITIVE,f32::from_bits(1),f32::MAX,f32::MIN,0.0625,0.5];
    let mut seed=0x1759_bacd_0145_b87d;
    for index in 0..512 {
        let width=1+(next(&mut seed)%13) as usize;let height=1+(next(&mut seed)%11) as usize;
        let gw=1+(next(&mut seed)%9) as usize;let gh=1+(next(&mut seed)%7) as usize;
        let channels=if index%2==0{1}else{3};let stride=gw*channels+(index%5);
        let data:Vec<u8>=(0..stride*gh).map(|_|next(&mut seed) as u8).collect();
        let samples=(0..width*height*3).map(|i|if index%8==0{finite[i%finite.len()]}else{(unit(&mut seed)*5.0-0.5) as f32}).collect();
        let sdr=SceneLinearRgb{width,height,samples};let mut app=application();
        app.weight=[0.0,-0.0,f64::from_bits(1),0.25,0.5,1.0][index%6];
        for (i,c) in app.channels.iter_mut().enumerate(){
            c.minimum_log2=unit(&mut seed)*4.0-2.0;c.maximum_log2=unit(&mut seed)*8.0-1.0;
            c.gamma=[1.0,0.5,2.0,2.4,1e-300,f64::MAX,f64::from_bits(1)][(index+i)%7];
            c.base_offset=[0.0,-0.0,1.0/64.0,-0.125,1e300][(index+i)%5];
            c.alternate_offset=[0.0,-0.0,1.0/128.0,-0.25,-1e300][(index+i)%5];
        }
        run(&format!("generated-{index}"),&sdr,GainMapCodes{width:gw,height:gh,bytes_per_row:stride,channels,data:&data},&app);
    }
    let base=SceneLinearRgb{width:2,height:2,samples:vec![0.0,-0.0,0.25,1.0,0.5,0.75,0.125,2.0,4.0,8.0,-1.0,f32::from_bits(1)]};
    let gain_data=[0,17,255,128,11,200,255,1,42,100,240,77];
    let gain=GainMapCodes{width:2,height:2,bytes_per_row:6,channels:3,data:&gain_data};
    let bads=[f64::NAN,f64::from_bits(0x7ff8_0000_0000_0001),f64::INFINITY,f64::NEG_INFINITY];
    for (j,bad) in bads.into_iter().enumerate(){for component in 0..3{for field in 0..5{
        let mut app=application();let c=&mut app.channels[component];match field{0=>c.minimum_log2=bad,1=>c.maximum_log2=bad,2=>c.gamma=bad,3=>c.base_offset=bad,_=>c.alternate_offset=bad};
        app.weight=f64::NAN; // Invalid channel must still precede invalid weight.
        run(&format!("bad-channel-{j}-{component}-{field}"),&base,gain,&app);
    }}}
    for gamma in [0.0,-0.0,-1.0,-f64::MIN_POSITIVE,-f64::from_bits(1)]{let mut app=application();app.channels[1].gamma=gamma;run("nonpositive-gamma",&base,gain,&app);}
    for weight in [f64::NAN,f64::INFINITY,f64::NEG_INFINITY,-1.0,1.0000000000000002,-f64::from_bits(1)]{let mut app=application();app.weight=weight;run("bad-weight",&base,gain,&app);}
    for bad in [f32::NAN,f32::INFINITY,f32::NEG_INFINITY] {let mut image=base.clone();image.samples[9]=bad;let mut app=application();app.weight=f64::NAN;run("bad-source-before-weight",&image,gain,&app);}
    for image in [SceneLinearRgb{width:0,height:2,samples:vec![]},SceneLinearRgb{width:2,height:0,samples:vec![]},SceneLinearRgb{width:usize::MAX,height:2,samples:vec![]},SceneLinearRgb{width:usize::MAX/2,height:1,samples:vec![]},SceneLinearRgb{width:2,height:2,samples:vec![0.0;11]},SceneLinearRgb{width:2,height:2,samples:vec![0.0;13]}] {
        let mut app=application();app.channels[0].gamma=0.0;run("bad-source-before-gain-channel",&image,GainMapCodes{width:0,..gain},&app);
    }
    for bad_gain in [GainMapCodes{width:0,..gain},GainMapCodes{height:0,..gain},GainMapCodes{channels:0,..gain},GainMapCodes{channels:2,..gain},GainMapCodes{channels:4,..gain},GainMapCodes{width:usize::MAX,channels:3,..gain},GainMapCodes{bytes_per_row:5,..gain},GainMapCodes{bytes_per_row:usize::MAX,height:2,..gain},GainMapCodes{data:&gain_data[..11],..gain}] {
        let mut app=application();app.channels[0].gamma=0.0;run("bad-gain-before-channel",&base,bad_gain,&app);
    }
    for (name,min,max,offset) in [("f64-overflow",1024.0,1024.0,0.0),("f32-overflow",200.0,200.0,0.0),("log-span-overflow",-f64::MAX,f64::MAX,0.0),("offset-overflow",0.0,1.0,f64::MAX),("finite-underflow",-2000.0,-2000.0,0.0),("zero-weight-log-span",-f64::MAX,f64::MAX,0.0)] {
        let mut app=application();for c in &mut app.channels{c.minimum_log2=min;c.maximum_log2=max;c.base_offset=offset;c.alternate_offset=0.0;}if name=="zero-weight-log-span"{app.weight=0.0;}run(name,&base,gain,&app);
    }
    drop(run);
    let mut scalar_channels=vec![channel()];
    for field in 0..5 {for bad in [f64::NAN,f64::INFINITY,f64::NEG_INFINITY,0.0,-0.0,-1.0,f64::MIN_POSITIVE,f64::from_bits(1),f64::MAX,-f64::MAX] {
        let mut c=channel();match field{0=>c.minimum_log2=bad,1=>c.maximum_log2=bad,2=>c.gamma=bad,3=>c.base_offset=bad,_=>c.alternate_offset=bad};scalar_channels.push(c);
    }}
    let scalar_values=[0.0,-0.0,0.25,-1.0,1.0,f64::MIN_POSITIVE,f64::from_bits(1),f64::MAX,-f64::MAX,f64::NAN,f64::INFINITY,f64::NEG_INFINITY];
    let weights=[0.0,-0.0,0.5,1.0,-1.0,1.0000000000000002,f64::NAN,f64::INFINITY,f64::NEG_INFINITY];
    let mut scalar_count=0;let mut scalar_ok=0;let mut scalar_err=0;
    for c in &scalar_channels {for sdr in scalar_values {for code in scalar_values {for weight in weights {
        scalar_count+=1;if scalar_record(&mut out,sdr,code,c,weight){scalar_ok+=1;}else{scalar_err+=1;}
    }}}}
    out.flush().unwrap();
    println!("reconstruction_cases={reconstructions} ok={reconstruction_ok} error={reconstruction_err} scalar_cases={scalar_count} scalar_ok={scalar_ok} scalar_error={scalar_err}");
}
fn read_u64(bytes:&[u8],cursor:&mut usize)->u64 {let result=u64::from_le_bytes(bytes[*cursor..*cursor+8].try_into().unwrap());*cursor+=8;result}
fn read_f64(bytes:&[u8],cursor:&mut usize)->f64 {f64::from_bits(read_u64(bytes,cursor))}
fn real(input:&str,output:&str) {
    let bytes=fs::read(input).unwrap();assert_eq!(&bytes[..8],b"HDRPROBE");let mut cursor=8;
    let width=read_u64(&bytes,&mut cursor) as usize;let height=read_u64(&bytes,&mut cursor) as usize;
    let gw=read_u64(&bytes,&mut cursor) as usize;let gh=read_u64(&bytes,&mut cursor) as usize;
    let stride=read_u64(&bytes,&mut cursor) as usize;let channels=read_u64(&bytes,&mut cursor) as usize;let gain_len=read_u64(&bytes,&mut cursor) as usize;
    let mut app=application();for c in &mut app.channels {c.minimum_log2=read_f64(&bytes,&mut cursor);c.maximum_log2=read_f64(&bytes,&mut cursor);c.gamma=read_f64(&bytes,&mut cursor);c.base_offset=read_f64(&bytes,&mut cursor);c.alternate_offset=read_f64(&bytes,&mut cursor);}app.weight=read_f64(&bytes,&mut cursor);
    let mut samples=Vec::with_capacity(width*height*3);for _ in 0..width*height*3 {samples.push(f32::from_le_bytes(bytes[cursor..cursor+4].try_into().unwrap()));cursor+=4;}
    let sdr=SceneLinearRgb{width,height,samples};let gain=&bytes[cursor..];assert_eq!(gain.len(),gain_len);
    let gain=GainMapCodes{width:gw,height:gh,bytes_per_row:stride,channels,data:gain};
    let start=Instant::now();let result=reconstruct_scene_linear_hdr(black_box(&sdr),black_box(gain),black_box(&app));let elapsed=start.elapsed().as_secs_f64();
    let mut out=BufWriter::new(fs::File::create(output).unwrap());let ok=record(&mut out,"public-source-kernel",result);out.flush().unwrap();assert!(ok,"real public fixture reconstruction failed");
    println!("kernel_wall_seconds={elapsed:.9} width={width} height={height} samples={} gain_width={gw} gain_height={gh} gain_channels={channels}",width*height*3);
}
fn main(){let args:Vec<_>=std::env::args().collect();match args.get(1).map(String::as_str){Some("matrix")=>matrix(&args[2]),Some("real")=>real(&args[2],&args[3]),_=>panic!("matrix OUTPUT or real INPUT OUTPUT")}}
