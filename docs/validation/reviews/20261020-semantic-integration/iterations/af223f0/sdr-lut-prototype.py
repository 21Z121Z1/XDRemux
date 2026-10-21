"""Audit-only exhaustive RGB8 decode equivalence and real-fixture kernel timing.

No product source changes, native execution claim, or source/media upload.
"""
from pathlib import Path
import subprocess,hashlib,json,os

repo=Path('/workspace/XDRemux-integration');root=Path('/workspace/xdremux-audit/sdr-lut-prototype')
root.mkdir(exist_ok=False)
source=repo/'crates/xdremux-runtime/src/apple_styles.rs'
text=source.read_text();start=text.index('fn srgb_decode(value: f32) -> f32 {');end=text.index('\nfn luminance',start)
function=text[start:end].strip()
(root/'original-srgb-decode.rs').write_text(function+'\n')
c=r'''
#include <libheif/heif.h>
#include <stdio.h>
#include <stdlib.h>
static void check(struct heif_error e) { if(e.code) { fprintf(stderr,"heif: %d/%d %s\n",e.code,e.subcode,e.message); exit(2); } }
int main(int argc,char**argv) {
 if(argc!=3)return 2;
 struct heif_context *c=heif_context_alloc();struct heif_image_handle *h=NULL;struct heif_image *im=NULL;
 check(heif_context_read_from_file(c,argv[1],NULL));check(heif_context_get_primary_image_handle(c,&h));
 struct heif_decoding_options *o=heif_decoding_options_alloc();o->ignore_transformations=1;
 check(heif_decode_image(h,&im,heif_colorspace_RGB,heif_chroma_interleaved_RGB,o));
 int w=heif_image_get_width(im,heif_channel_interleaved),ht=heif_image_get_height(im,heif_channel_interleaved),stride=0;
 const unsigned char *p=heif_image_get_plane_readonly(im,heif_channel_interleaved,&stride);
 if(!p||w<=0||ht<=0||stride<(long long)w*3)return 3;
 FILE*f=fopen(argv[2],"wx");if(!f)return 4;
 for(int y=0;y<ht;y++)if(fwrite(p+(size_t)y*stride,3,(size_t)w,f)!=(size_t)w)return 5;
 if(fclose(f))return 6;
 printf("{\"width\":%d,\"height\":%d,\"source_stride\":%d,\"packed_components\":%lld,\"ignore_transforms\":true}\n",w,ht,stride,(long long)w*ht*3);
 heif_decoding_options_free(o);heif_image_release(im);heif_image_handle_release(h);heif_context_free(c);return 0;
}
'''
(root/'decode.c').write_text(c)
env={**os.environ,'PKG_CONFIG_PATH':'/tmp/xdremux-toolchain/heif-dev/usr/lib/x86_64-linux-gnu/pkgconfig',
     'LIBHEIF_PLUGIN_PATH':'/tmp/xdremux-toolchain/heif-dev/usr/lib/x86_64-linux-gnu/libheif/plugins'}
def run(name,args,timeout=120):
 r=subprocess.run(list(map(str,args)),cwd=repo,env=env,capture_output=True,timeout=timeout)
 (root/(name+'.stdout')).write_bytes(r.stdout);(root/(name+'.stderr')).write_bytes(r.stderr)
 assert r.returncode==0,(name,r.returncode,r.stderr.decode(errors='replace'));return r.stdout
flags=run('pkg-config',['pkg-config','--cflags','--libs','libheif']).decode().split()
run('compile-decoder',['cc','-O2',root/'decode.c',*flags,'-o',root/'decode'])
fixture=repo/'fixtures/proxdr/oppo/find-x9-ultra/uhdr-hr-01.heic'
decoded=json.loads(run('decode',[root/'decode',fixture,root/'input.rgb8']))
rust=r'''
use std::{hint::black_box,time::Instant};
@@FUNCTION@@
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
'''.replace('@@FUNCTION@@',function)
(root/'kernel.rs').write_text(rust)
rustc='/tmp/xdremux-toolchain/rustup/toolchains/stable-x86_64-unknown-linux-gnu/bin/rustc'
version=run('rust-version',[rustc,'-vV']).decode()
run('compile-kernel',[rustc,'--edition=2024','-O','-C','target-cpu=native',root/'kernel.rs','-o',root/'kernel'])
result=json.loads(run('kernel',[root/'kernel',root/'input.rgb8']))
import statistics
medians={arm:statistics.median(row['seconds'] for row in result['samples'] if row['arm']==arm) for arm in ['baseline','lookup']}
report={'source':subprocess.check_output(['git','rev-parse','HEAD'],cwd=repo,text=True).strip(),
        'source_path':str(source.relative_to(repo)),'source_file_sha256':hashlib.sha256(source.read_bytes()).hexdigest(),
        'copied_function':function,'fixture':str(fixture.relative_to(repo)),'fixture_sha256':hashlib.sha256(fixture.read_bytes()).hexdigest(),
        'decoded':decoded,'decoded_rgb8_sha256':hashlib.sha256((root/'input.rgb8').read_bytes()).hexdigest(),
        'rustc':version,'kernel':result,'medians':medians,'lookup_over_baseline':medians['lookup']/medians['baseline'],
        'scope':'Linux x86_64 standalone kernel only, not runtime row/pixel validation or E2E conversion. Table initialization and output allocation included, deallocation excluded. One warmup/arm;8alternating-order samples/arm.',
        'limits':['No arm64/macOS timing, native conversion, or whole-gap attribution.','No product changes adopted or budget/validation changes.','RGB8 decoded locally from exact public fixture; no source media/decoded pixels in public artifacts.']}
(root/'result.json').write_text(json.dumps(report,indent=2)+'\n');print(json.dumps(report,indent=2))
