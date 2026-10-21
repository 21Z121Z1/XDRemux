
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
