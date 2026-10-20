
/tmp/xdremux-style-fit-accumulation-target/candidate/release/xdremux-style-fit-probe:     file format elf64-x86-64


Disassembly of section .text:

0000000000027890 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data>:
   27890:	55                   	push   %rbp
   27891:	41 57                	push   %r15
   27893:	41 56                	push   %r14
   27895:	41 55                	push   %r13
   27897:	41 54                	push   %r12
   27899:	53                   	push   %rbx
   2789a:	48 81 ec 78 02 00 00 	sub    $0x278,%rsp
   278a1:	4c 8b 76 20          	mov    0x20(%rsi),%r14
   278a5:	48 8b 4e 28          	mov    0x28(%rsi),%rcx
   278a9:	4c 89 f0             	mov    %r14,%rax
   278ac:	48 f7 e1             	mul    %rcx
   278af:	0f 80 db 17 00 00    	jo     29090 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1800>
   278b5:	48 89 8c 24 70 01 00 	mov    %rcx,0x170(%rsp)
   278bc:	00 
   278bd:	b9 03 00 00 00       	mov    $0x3,%ecx
   278c2:	48 f7 e1             	mul    %rcx
   278c5:	0f 80 c5 17 00 00    	jo     29090 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1800>
   278cb:	4d 85 f6             	test   %r14,%r14
   278ce:	0f 84 bc 17 00 00    	je     29090 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1800>
   278d4:	48 83 bc 24 70 01 00 	cmpq   $0x0,0x170(%rsp)
   278db:	00 00 
   278dd:	0f 84 ad 17 00 00    	je     29090 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1800>
   278e3:	49 89 c7             	mov    %rax,%r15
   278e6:	48 39 46 08          	cmp    %rax,0x8(%rsi)
   278ea:	0f 85 a0 17 00 00    	jne    29090 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1800>
   278f0:	4c 39 7e 18          	cmp    %r15,0x18(%rsi)
   278f4:	0f 85 96 17 00 00    	jne    29090 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1800>
   278fa:	4c 8b 26             	mov    (%rsi),%r12
   278fd:	4a 8d 04 bd 00 00 00 	lea    0x0(,%r15,4),%rax
   27904:	00 
   27905:	4d 85 ff             	test   %r15,%r15
   27908:	74 21                	je     2792b <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x9b>
   2790a:	31 c9                	xor    %ecx,%ecx
   2790c:	0f 1f 40 00          	nopl   0x0(%rax)
   27910:	41 8b 14 0c          	mov    (%r12,%rcx,1),%edx
   27914:	01 d2                	add    %edx,%edx
   27916:	81 fa ff ff ff fe    	cmp    $0xfeffffff,%edx
   2791c:	0f 87 6e 17 00 00    	ja     29090 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1800>
   27922:	48 83 c1 04          	add    $0x4,%rcx
   27926:	48 39 c8             	cmp    %rcx,%rax
   27929:	75 e5                	jne    27910 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x80>
   2792b:	48 8b 5e 10          	mov    0x10(%rsi),%rbx
   2792f:	4d 85 ff             	test   %r15,%r15
   27932:	74 26                	je     2795a <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0xca>
   27934:	31 c9                	xor    %ecx,%ecx
   27936:	66 2e 0f 1f 84 00 00 	cs nopw 0x0(%rax,%rax,1)
   2793d:	00 00 00 
   27940:	8b 14 0b             	mov    (%rbx,%rcx,1),%edx
   27943:	01 d2                	add    %edx,%edx
   27945:	81 fa ff ff ff fe    	cmp    $0xfeffffff,%edx
   2794b:	0f 87 3f 17 00 00    	ja     29090 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1800>
   27951:	48 83 c1 04          	add    $0x4,%rcx
   27955:	48 39 c8             	cmp    %rcx,%rax
   27958:	75 e6                	jne    27940 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0xb0>
   2795a:	48 89 bc 24 10 01 00 	mov    %rdi,0x110(%rsp)
   27961:	00 
   27962:	ff 15 b0 bd 04 00    	call   *0x4bdb0(%rip)        # 73718 <_DYNAMIC+0x288>
   27968:	bf 00 8c 0a 00       	mov    $0xa8c00,%edi
   2796d:	be 08 00 00 00       	mov    $0x8,%esi
   27972:	ff 15 e0 be 04 00    	call   *0x4bee0(%rip)        # 73858 <_DYNAMIC+0x3c8>
   27978:	48 89 44 24 18       	mov    %rax,0x18(%rsp)
   2797d:	48 85 c0             	test   %rax,%rax
   27980:	0f 84 3a 1f 00 00    	je     298c0 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x2030>
   27986:	ff 15 8c bd 04 00    	call   *0x4bd8c(%rip)        # 73718 <_DYNAMIC+0x288>
   2798c:	bf 00 2a 03 00       	mov    $0x32a00,%edi
   27991:	be 08 00 00 00       	mov    $0x8,%esi
   27996:	ff 15 bc be 04 00    	call   *0x4bebc(%rip)        # 73858 <_DYNAMIC+0x3c8>
   2799c:	48 85 c0             	test   %rax,%rax
   2799f:	0f 84 2b 1f 00 00    	je     298d0 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x2040>
   279a5:	48 c7 44 24 78 40 65 	movq   $0x6540,0x78(%rsp)
   279ac:	00 00 
   279ae:	48 89 84 24 80 00 00 	mov    %rax,0x80(%rsp)
   279b5:	00 
   279b6:	48 c7 84 24 88 00 00 	movq   $0x6540,0x88(%rsp)
   279bd:	00 40 65 00 00 
   279c2:	48 8b 44 24 18       	mov    0x18(%rsp),%rax
   279c7:	48 83 c0 08          	add    $0x8,%rax
   279cb:	48 89 84 24 18 02 00 	mov    %rax,0x218(%rsp)
   279d2:	00 
   279d3:	45 31 ed             	xor    %r13d,%r13d
   279d6:	48 8d ac 24 30 01 00 	lea    0x130(%rsp),%rbp
   279dd:	00 
   279de:	48 8d 05 73 97 04 00 	lea    0x49773(%rip),%rax        # 71158 <anon.e11926f7acc299ef6543ba90fd675eca.18.llvm.1662880743899104407+0x1f8>
   279e5:	48 89 84 24 68 01 00 	mov    %rax,0x168(%rsp)
   279ec:	00 
   279ed:	4c 89 bc 24 a8 01 00 	mov    %r15,0x1a8(%rsp)
   279f4:	00 
   279f5:	48 8b 94 24 70 01 00 	mov    0x170(%rsp),%rdx
   279fc:	00 
   279fd:	4c 89 b4 24 28 01 00 	mov    %r14,0x128(%rsp)
   27a04:	00 
   27a05:	4c 89 a4 24 28 02 00 	mov    %r12,0x228(%rsp)
   27a0c:	00 
   27a0d:	48 89 9c 24 20 02 00 	mov    %rbx,0x220(%rsp)
   27a14:	00 
   27a15:	b9 09 00 00 00       	mov    $0x9,%ecx
   27a1a:	48 89 ef             	mov    %rbp,%rdi
   27a1d:	4c 89 ee             	mov    %r13,%rsi
   27a20:	e8 1b ec ff ff       	call   26640 <xdremux_engine::apple_photographic_styles::spotlight_weights>
   27a25:	49 8d 45 01          	lea    0x1(%r13),%rax
   27a29:	48 89 84 24 10 02 00 	mov    %rax,0x210(%rsp)
   27a30:	00 
   27a31:	4d 0f af ee          	imul   %r14,%r13
   27a35:	48 8b 84 24 30 01 00 	mov    0x130(%rsp),%rax
   27a3c:	00 
   27a3d:	f2 0f 10 84 24 38 01 	movsd  0x138(%rsp),%xmm0
   27a44:	00 00 
   27a46:	f2 0f 11 84 24 20 01 	movsd  %xmm0,0x120(%rsp)
   27a4d:	00 00 
   27a4f:	48 8b 8c 24 40 01 00 	mov    0x140(%rsp),%rcx
   27a56:	00 
   27a57:	f2 0f 10 84 24 48 01 	movsd  0x148(%rsp),%xmm0
   27a5e:	00 00 
   27a60:	f2 0f 11 84 24 18 01 	movsd  %xmm0,0x118(%rsp)
   27a67:	00 00 
   27a69:	48 89 84 24 c8 01 00 	mov    %rax,0x1c8(%rsp)
   27a70:	00 
   27a71:	48 69 d0 00 19 00 00 	imul   $0x1900,%rax,%rdx
   27a78:	48 8b 84 24 18 02 00 	mov    0x218(%rsp),%rax
   27a7f:	00 
   27a80:	48 01 c2             	add    %rax,%rdx
   27a83:	48 89 94 24 c0 01 00 	mov    %rdx,0x1c0(%rsp)
   27a8a:	00 
   27a8b:	48 89 8c 24 b8 01 00 	mov    %rcx,0x1b8(%rsp)
   27a92:	00 
   27a93:	48 69 c9 00 19 00 00 	imul   $0x1900,%rcx,%rcx
   27a9a:	48 01 c1             	add    %rax,%rcx
   27a9d:	48 89 8c 24 b0 01 00 	mov    %rcx,0x1b0(%rsp)
   27aa4:	00 
   27aa5:	31 c9                	xor    %ecx,%ecx
   27aa7:	4c 89 6c 24 20       	mov    %r13,0x20(%rsp)
   27aac:	eb 33                	jmp    27ae1 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x251>
   27aae:	66 90                	xchg   %ax,%ax
   27ab0:	4c 8b bc 24 a8 01 00 	mov    0x1a8(%rsp),%r15
   27ab7:	00 
   27ab8:	4c 8b b4 24 28 01 00 	mov    0x128(%rsp),%r14
   27abf:	00 
   27ac0:	4c 8b a4 24 28 02 00 	mov    0x228(%rsp),%r12
   27ac7:	00 
   27ac8:	48 8b 9c 24 20 02 00 	mov    0x220(%rsp),%rbx
   27acf:	00 
   27ad0:	48 8b 4c 24 70       	mov    0x70(%rsp),%rcx
   27ad5:	48 ff c1             	inc    %rcx
   27ad8:	4c 39 f1             	cmp    %r14,%rcx
   27adb:	0f 84 a6 0b 00 00    	je     28687 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0xdf7>
   27ae1:	4a 8d 04 29          	lea    (%rcx,%r13,1),%rax
   27ae5:	48 8d 3c 40          	lea    (%rax,%rax,2),%rdi
   27ae9:	4c 39 ff             	cmp    %r15,%rdi
   27aec:	0f 83 96 1d 00 00    	jae    29888 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1ff8>
   27af2:	48 8d 47 01          	lea    0x1(%rdi),%rax
   27af6:	4c 39 f8             	cmp    %r15,%rax
   27af9:	0f 83 6b 1d 00 00    	jae    2986a <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1fda>
   27aff:	4c 89 fa             	mov    %r15,%rdx
   27b02:	48 89 4c 24 70       	mov    %rcx,0x70(%rsp)
   27b07:	48 8d 47 02          	lea    0x2(%rdi),%rax
   27b0b:	4c 39 f8             	cmp    %r15,%rax
   27b0e:	0f 83 62 1d 00 00    	jae    29876 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1fe6>
   27b14:	49 89 d7             	mov    %rdx,%r15
   27b17:	f3 41 0f 10 04 bc    	movss  (%r12,%rdi,4),%xmm0
   27b1d:	0f 57 e4             	xorps  %xmm4,%xmm4
   27b20:	f3 0f 5a e0          	cvtss2sd %xmm0,%xmm4
   27b24:	0f 5a 04 bb          	cvtps2pd (%rbx,%rdi,4),%xmm0
   27b28:	0f 29 04 24          	movaps %xmm0,(%rsp)
   27b2c:	f3 0f 10 44 bb 08    	movss  0x8(%rbx,%rdi,4),%xmm0
   27b32:	f3 0f 11 84 24 78 01 	movss  %xmm0,0x178(%rsp)
   27b39:	00 00 
   27b3b:	0f 28 c4             	movaps %xmm4,%xmm0
   27b3e:	48 b8 00 00 00 00 00 	movabs $0x3ff0000000000000,%rax
   27b45:	00 f0 3f 
   27b48:	48 89 84 24 b0 00 00 	mov    %rax,0xb0(%rsp)
   27b4f:	00 
   27b50:	f2 0f 11 a4 24 b8 00 	movsd  %xmm4,0xb8(%rsp)
   27b57:	00 00 
   27b59:	41 0f 5a 54 bc 04    	cvtps2pd 0x4(%r12,%rdi,4),%xmm2
   27b5f:	f2 0f 59 c4          	mulsd  %xmm4,%xmm0
   27b63:	66 0f 28 ea          	movapd %xmm2,%xmm5
   27b67:	66 0f 15 ea          	unpckhpd %xmm2,%xmm5
   27b6b:	66 0f 28 cd          	movapd %xmm5,%xmm1
   27b6f:	f2 0f 59 cd          	mulsd  %xmm5,%xmm1
   27b73:	66 0f 11 94 24 c0 00 	movupd %xmm2,0xc0(%rsp)
   27b7a:	00 00 
   27b7c:	f2 0f 11 84 24 d0 00 	movsd  %xmm0,0xd0(%rsp)
   27b83:	00 00 
   27b85:	66 0f 28 c4          	movapd %xmm4,%xmm0
   27b89:	66 0f 14 c4          	unpcklpd %xmm4,%xmm0
   27b8d:	66 0f 59 c2          	mulpd  %xmm2,%xmm0
   27b91:	66 0f 11 84 24 d8 00 	movupd %xmm0,0xd8(%rsp)
   27b98:	00 00 
   27b9a:	66 0f 28 c2          	movapd %xmm2,%xmm0
   27b9e:	66 0f 14 c2          	unpcklpd %xmm2,%xmm0
   27ba2:	66 0f 59 c2          	mulpd  %xmm2,%xmm0
   27ba6:	66 0f 11 84 24 e8 00 	movupd %xmm0,0xe8(%rsp)
   27bad:	00 00 
   27baf:	f2 0f 11 8c 24 f8 00 	movsd  %xmm1,0xf8(%rsp)
   27bb6:	00 00 
   27bb8:	66 0f 28 c2          	movapd %xmm2,%xmm0
   27bbc:	f2 0f 58 c4          	addsd  %xmm4,%xmm0
   27bc0:	f2 0f 58 c5          	addsd  %xmm5,%xmm0
   27bc4:	f2 0f 5e 05 9c 51 fe 	divsd  -0x1ae64(%rip),%xmm0        # cd68 <GCC_except_table0+0xdc>
   27bcb:	ff 
   27bcc:	66 0f 28 cc          	movapd %xmm4,%xmm1
   27bd0:	f2 0f 5f ca          	maxsd  %xmm2,%xmm1
   27bd4:	66 0f 29 94 24 d0 01 	movapd %xmm2,0x1d0(%rsp)
   27bdb:	00 00 
   27bdd:	f2 0f c2 d2 03       	cmpunordsd %xmm2,%xmm2
   27be2:	66 0f 28 da          	movapd %xmm2,%xmm3
   27be6:	66 0f 55 d9          	andnpd %xmm1,%xmm3
   27bea:	0f 29 a4 24 f0 01 00 	movaps %xmm4,0x1f0(%rsp)
   27bf1:	00 
   27bf2:	66 0f 54 d4          	andpd  %xmm4,%xmm2
   27bf6:	66 0f 56 d3          	orpd   %xmm3,%xmm2
   27bfa:	66 0f 28 dd          	movapd %xmm5,%xmm3
   27bfe:	f2 0f c2 dd 03       	cmpunordsd %xmm5,%xmm3
   27c03:	66 0f 28 cb          	movapd %xmm3,%xmm1
   27c07:	66 0f 54 ca          	andpd  %xmm2,%xmm1
   27c0b:	66 0f 29 ac 24 e0 01 	movapd %xmm5,0x1e0(%rsp)
   27c12:	00 00 
   27c14:	f2 0f 5f d5          	maxsd  %xmm5,%xmm2
   27c18:	66 0f 55 da          	andnpd %xmm2,%xmm3
   27c1c:	66 0f 56 d9          	orpd   %xmm1,%xmm3
   27c20:	f2 0f 58 d8          	addsd  %xmm0,%xmm3
   27c24:	f2 0f 59 1d 1c 51 fe 	mulsd  -0x1aee4(%rip),%xmm3        # cd48 <GCC_except_table0+0xbc>
   27c2b:	ff 
   27c2c:	66 0f 14 db          	unpcklpd %xmm3,%xmm3
   27c30:	66 0f 29 5c 24 50    	movapd %xmm3,0x50(%rsp)
   27c36:	66 0f 58 1d f2 53 fe 	addpd  -0x1ac0e(%rip),%xmm3        # d030 <GCC_except_table0+0x3a4>
   27c3d:	ff 
   27c3e:	66 0f 28 05 5a 53 fe 	movapd -0x1aca6(%rip),%xmm0        # cfa0 <GCC_except_table0+0x314>
   27c45:	ff 
   27c46:	66 0f 59 d8          	mulpd  %xmm0,%xmm3
   27c4a:	66 0f 28 05 6e 56 fe 	movapd -0x1a992(%rip),%xmm0        # d2c0 <GCC_except_table0+0x634>
   27c51:	ff 
   27c52:	66 0f 5e d8          	divpd  %xmm0,%xmm3
   27c56:	66 0f 59 db          	mulpd  %xmm3,%xmm3
   27c5a:	66 0f 28 05 ce 57 fe 	movapd -0x1a832(%rip),%xmm0        # d430 <GCC_except_table0+0x7a4>
   27c61:	ff 
   27c62:	66 0f 59 d8          	mulpd  %xmm0,%xmm3
   27c66:	66 0f 29 9c 24 90 00 	movapd %xmm3,0x90(%rsp)
   27c6d:	00 00 
   27c6f:	66 0f 28 c3          	movapd %xmm3,%xmm0
   27c73:	4c 8b 35 26 be 04 00 	mov    0x4be26(%rip),%r14        # 73aa0 <exp@GLIBC_2.29>
   27c7a:	41 ff d6             	call   *%r14
   27c7d:	66 0f 29 44 24 60    	movapd %xmm0,0x60(%rsp)
   27c83:	0f 28 84 24 90 00 00 	movaps 0x90(%rsp),%xmm0
   27c8a:	00 
   27c8b:	0f 12 c0             	movhlps %xmm0,%xmm0
   27c8e:	41 ff d6             	call   *%r14
   27c91:	0f 29 84 24 90 01 00 	movaps %xmm0,0x190(%rsp)
   27c98:	00 
   27c99:	0f 28 4c 24 60       	movaps 0x60(%rsp),%xmm1
   27c9e:	0f 16 c8             	movlhps %xmm0,%xmm1
   27ca1:	0f 29 4c 24 60       	movaps %xmm1,0x60(%rsp)
   27ca6:	0f 29 8c 24 30 02 00 	movaps %xmm1,0x230(%rsp)
   27cad:	00 
   27cae:	66 0f 28 44 24 50    	movapd 0x50(%rsp),%xmm0
   27cb4:	66 0f 58 05 54 54 fe 	addpd  -0x1abac(%rip),%xmm0        # d110 <GCC_except_table0+0x484>
   27cbb:	ff 
   27cbc:	66 0f 59 05 dc 52 fe 	mulpd  -0x1ad24(%rip),%xmm0        # cfa0 <GCC_except_table0+0x314>
   27cc3:	ff 
   27cc4:	66 0f 5e 05 f4 55 fe 	divpd  -0x1aa0c(%rip),%xmm0        # d2c0 <GCC_except_table0+0x634>
   27ccb:	ff 
   27ccc:	66 0f 59 c0          	mulpd  %xmm0,%xmm0
   27cd0:	66 0f 59 05 58 57 fe 	mulpd  -0x1a8a8(%rip),%xmm0        # d430 <GCC_except_table0+0x7a4>
   27cd7:	ff 
   27cd8:	66 0f 29 84 24 00 01 	movapd %xmm0,0x100(%rsp)
   27cdf:	00 00 
   27ce1:	41 ff d6             	call   *%r14
   27ce4:	66 0f 29 84 24 90 00 	movapd %xmm0,0x90(%rsp)
   27ceb:	00 00 
   27ced:	0f 28 84 24 00 01 00 	movaps 0x100(%rsp),%xmm0
   27cf4:	00 
   27cf5:	0f 12 c0             	movhlps %xmm0,%xmm0
   27cf8:	41 ff d6             	call   *%r14
   27cfb:	0f 29 84 24 80 01 00 	movaps %xmm0,0x180(%rsp)
   27d02:	00 
   27d03:	0f 28 8c 24 90 00 00 	movaps 0x90(%rsp),%xmm1
   27d0a:	00 
   27d0b:	0f 16 c8             	movlhps %xmm0,%xmm1
   27d0e:	0f 29 8c 24 90 00 00 	movaps %xmm1,0x90(%rsp)
   27d15:	00 
   27d16:	0f 29 8c 24 40 02 00 	movaps %xmm1,0x240(%rsp)
   27d1d:	00 
   27d1e:	66 0f 28 44 24 50    	movapd 0x50(%rsp),%xmm0
   27d24:	66 0f 58 05 b4 51 fe 	addpd  -0x1ae4c(%rip),%xmm0        # cee0 <GCC_except_table0+0x254>
   27d2b:	ff 
   27d2c:	66 0f 59 05 6c 52 fe 	mulpd  -0x1ad94(%rip),%xmm0        # cfa0 <GCC_except_table0+0x314>
   27d33:	ff 
   27d34:	66 0f 5e 05 84 55 fe 	divpd  -0x1aa7c(%rip),%xmm0        # d2c0 <GCC_except_table0+0x634>
   27d3b:	ff 
   27d3c:	66 0f 59 c0          	mulpd  %xmm0,%xmm0
   27d40:	66 0f 59 05 e8 56 fe 	mulpd  -0x1a918(%rip),%xmm0        # d430 <GCC_except_table0+0x7a4>
   27d47:	ff 
   27d48:	66 0f 29 84 24 50 01 	movapd %xmm0,0x150(%rsp)
   27d4f:	00 00 
   27d51:	41 ff d6             	call   *%r14
   27d54:	66 0f 29 84 24 00 01 	movapd %xmm0,0x100(%rsp)
   27d5b:	00 00 
   27d5d:	0f 28 84 24 50 01 00 	movaps 0x150(%rsp),%xmm0
   27d64:	00 
   27d65:	0f 12 c0             	movhlps %xmm0,%xmm0
   27d68:	41 ff d6             	call   *%r14
   27d6b:	0f 29 84 24 50 01 00 	movaps %xmm0,0x150(%rsp)
   27d72:	00 
   27d73:	0f 28 8c 24 00 01 00 	movaps 0x100(%rsp),%xmm1
   27d7a:	00 
   27d7b:	0f 16 c8             	movlhps %xmm0,%xmm1
   27d7e:	0f 29 8c 24 00 01 00 	movaps %xmm1,0x100(%rsp)
   27d85:	00 
   27d86:	0f 29 8c 24 50 02 00 	movaps %xmm1,0x250(%rsp)
   27d8d:	00 
   27d8e:	66 0f 28 44 24 50    	movapd 0x50(%rsp),%xmm0
   27d94:	66 0f 58 05 74 54 fe 	addpd  -0x1ab8c(%rip),%xmm0        # d210 <GCC_except_table0+0x584>
   27d9b:	ff 
   27d9c:	66 0f 59 05 fc 51 fe 	mulpd  -0x1ae04(%rip),%xmm0        # cfa0 <GCC_except_table0+0x314>
   27da3:	ff 
   27da4:	66 0f 5e 05 14 55 fe 	divpd  -0x1aaec(%rip),%xmm0        # d2c0 <GCC_except_table0+0x634>
   27dab:	ff 
   27dac:	66 0f 59 c0          	mulpd  %xmm0,%xmm0
   27db0:	66 0f 59 05 78 56 fe 	mulpd  -0x1a988(%rip),%xmm0        # d430 <GCC_except_table0+0x7a4>
   27db7:	ff 
   27db8:	66 0f 29 44 24 50    	movapd %xmm0,0x50(%rsp)
   27dbe:	41 ff d6             	call   *%r14
   27dc1:	66 0f 29 84 24 00 02 	movapd %xmm0,0x200(%rsp)
   27dc8:	00 00 
   27dca:	66 0f 28 44 24 50    	movapd 0x50(%rsp),%xmm0
   27dd0:	66 0f 15 c0          	unpckhpd %xmm0,%xmm0
   27dd4:	41 ff d6             	call   *%r14
   27dd7:	66 0f 28 8c 24 00 02 	movapd 0x200(%rsp),%xmm1
   27dde:	00 00 
   27de0:	66 0f 14 c8          	unpcklpd %xmm0,%xmm1
   27de4:	66 0f 29 8c 24 60 02 	movapd %xmm1,0x260(%rsp)
   27deb:	00 00 
   27ded:	66 0f 28 d1          	movapd %xmm1,%xmm2
   27df1:	66 0f 28 4c 24 60    	movapd 0x60(%rsp),%xmm1
   27df7:	f2 0f 58 8c 24 90 01 	addsd  0x190(%rsp),%xmm1
   27dfe:	00 00 
   27e00:	f2 0f 58 8c 24 90 00 	addsd  0x90(%rsp),%xmm1
   27e07:	00 00 
   27e09:	f2 0f 58 8c 24 80 01 	addsd  0x180(%rsp),%xmm1
   27e10:	00 00 
   27e12:	f2 0f 58 8c 24 00 01 	addsd  0x100(%rsp),%xmm1
   27e19:	00 00 
   27e1b:	f2 0f 58 8c 24 50 01 	addsd  0x150(%rsp),%xmm1
   27e22:	00 00 
   27e24:	f2 0f 58 ca          	addsd  %xmm2,%xmm1
   27e28:	f2 0f 58 c8          	addsd  %xmm0,%xmm1
   27e2c:	66 0f 57 c0          	xorpd  %xmm0,%xmm0
   27e30:	66 0f 29 4c 24 60    	movapd %xmm1,0x60(%rsp)
   27e36:	66 0f 2e c1          	ucomisd %xmm1,%xmm0
   27e3a:	4c 8b b4 24 28 01 00 	mov    0x128(%rsp),%r14
   27e41:	00 
   27e42:	0f 83 88 fc ff ff    	jae    27ad0 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x240>
   27e48:	b9 0c 00 00 00       	mov    $0xc,%ecx
   27e4d:	48 8d 7c 24 28       	lea    0x28(%rsp),%rdi
   27e52:	48 8b 74 24 70       	mov    0x70(%rsp),%rsi
   27e57:	4c 89 f2             	mov    %r14,%rdx
   27e5a:	e8 e1 e7 ff ff       	call   26640 <xdremux_engine::apple_photographic_styles::spotlight_weights>
   27e5f:	f3 0f 10 84 24 78 01 	movss  0x178(%rsp),%xmm0
   27e66:	00 00 
   27e68:	f3 0f 5a c0          	cvtss2sd %xmm0,%xmm0
   27e6c:	66 0f 28 94 24 f0 01 	movapd 0x1f0(%rsp),%xmm2
   27e73:	00 00 
   27e75:	66 0f 14 94 24 d0 01 	unpcklpd 0x1d0(%rsp),%xmm2
   27e7c:	00 00 
   27e7e:	66 0f 28 0c 24       	movapd (%rsp),%xmm1
   27e83:	66 0f 5c ca          	subpd  %xmm2,%xmm1
   27e87:	66 0f 29 0c 24       	movapd %xmm1,(%rsp)
   27e8c:	f2 0f 5c 84 24 e0 01 	subsd  0x1e0(%rsp),%xmm0
   27e93:	00 00 
   27e95:	f2 0f 10 54 24 30    	movsd  0x30(%rsp),%xmm2
   27e9b:	48 8b 44 24 28       	mov    0x28(%rsp),%rax
   27ea0:	48 89 84 24 90 00 00 	mov    %rax,0x90(%rsp)
   27ea7:	00 
   27ea8:	48 8d 0c c0          	lea    (%rax,%rax,8),%rcx
   27eac:	f2 0f 10 4c 24 40    	movsd  0x40(%rsp),%xmm1
   27eb2:	48 8b 44 24 38       	mov    0x38(%rsp),%rax
   27eb7:	48 89 44 24 50       	mov    %rax,0x50(%rsp)
   27ebc:	66 0f 28 da          	movapd %xmm2,%xmm3
   27ec0:	f2 0f 59 9c 24 20 01 	mulsd  0x120(%rsp),%xmm3
   27ec7:	00 00 
   27ec9:	66 0f 57 e4          	xorpd  %xmm4,%xmm4
   27ecd:	66 0f 2e e3          	ucomisd %xmm3,%xmm4
   27ed1:	0f 83 e9 01 00 00    	jae    280c0 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x830>
   27ed7:	48 8b 84 24 c8 01 00 	mov    0x1c8(%rsp),%rax
   27ede:	00 
   27edf:	4c 8d 04 08          	lea    (%rax,%rcx,1),%r8
   27ee3:	49 c1 e0 03          	shl    $0x3,%r8
   27ee7:	48 69 84 24 90 00 00 	imul   $0xe100,0x90(%rsp),%rax
   27eee:	00 00 e1 00 00 
   27ef3:	48 03 84 24 c0 01 00 	add    0x1c0(%rsp),%rax
   27efa:	00 
   27efb:	45 31 db             	xor    %r11d,%r11d
   27efe:	31 db                	xor    %ebx,%ebx
   27f00:	eb 25                	jmp    27f27 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x697>
   27f02:	66 66 66 66 66 2e 0f 	data16 data16 data16 data16 cs nopw 0x0(%rax,%rax,1)
   27f09:	1f 84 00 00 00 00 00 
   27f10:	49 83 c3 08          	add    $0x8,%r11
   27f14:	48 ff c3             	inc    %rbx
   27f17:	48 05 20 03 00 00    	add    $0x320,%rax
   27f1d:	49 83 fb 40          	cmp    $0x40,%r11
   27f21:	0f 84 99 01 00 00    	je     280c0 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x830>
   27f27:	f2 42 0f 10 a4 1c 30 	movsd  0x230(%rsp,%r11,1),%xmm4
   27f2e:	02 00 00 
   27f31:	f2 0f 59 e3          	mulsd  %xmm3,%xmm4
   27f35:	f2 0f 5e 64 24 60    	divsd  0x60(%rsp),%xmm4
   27f3b:	66 0f 57 ed          	xorpd  %xmm5,%xmm5
   27f3f:	66 0f 2e ec          	ucomisd %xmm4,%xmm5
   27f43:	73 cb                	jae    27f10 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x680>
   27f45:	4a 8d 3c 03          	lea    (%rbx,%r8,1),%rdi
   27f49:	4c 6b f7 64          	imul   $0x64,%rdi,%r14
   27f4d:	49 81 fe 1c 51 01 00 	cmp    $0x1511c,%r14
   27f54:	0f 87 4e 11 00 00    	ja     290a8 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1818>
   27f5a:	48 8d 14 3f          	lea    (%rdi,%rdi,1),%rdx
   27f5e:	48 c1 e7 05          	shl    $0x5,%rdi
   27f62:	48 29 d7             	sub    %rdx,%rdi
   27f65:	48 8d 77 1e          	lea    0x1e(%rdi),%rsi
   27f69:	48 8b 94 24 88 00 00 	mov    0x88(%rsp),%rdx
   27f70:	00 
   27f71:	48 83 ff e1          	cmp    $0xffffffffffffffe1,%rdi
   27f75:	0f 87 42 11 00 00    	ja     290bd <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x182d>
   27f7b:	48 39 d6             	cmp    %rdx,%rsi
   27f7e:	0f 87 39 11 00 00    	ja     290bd <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x182d>
   27f84:	48 8b 54 24 18       	mov    0x18(%rsp),%rdx
   27f89:	4a 8d 14 f2          	lea    (%rdx,%r14,8),%rdx
   27f8d:	48 c1 e7 03          	shl    $0x3,%rdi
   27f91:	48 03 bc 24 80 00 00 	add    0x80(%rsp),%rdi
   27f98:	00 
   27f99:	be 0a 00 00 00       	mov    $0xa,%esi
   27f9e:	4c 8d a4 24 b8 00 00 	lea    0xb8(%rsp),%r12
   27fa5:	00 
   27fa6:	49 89 c2             	mov    %rax,%r10
   27fa9:	45 31 f6             	xor    %r14d,%r14d
   27fac:	e9 8e 00 00 00       	jmp    2803f <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x7af>
   27fb1:	66 66 66 66 66 66 2e 	data16 data16 data16 data16 data16 cs nopw 0x0(%rax,%rax,1)
   27fb8:	0f 1f 84 00 00 00 00 
   27fbf:	00 
   27fc0:	45 31 c9             	xor    %r9d,%r9d
   27fc3:	4f 8d 3c b6          	lea    (%r14,%r14,4),%r15
   27fc7:	49 c1 e7 04          	shl    $0x4,%r15
   27fcb:	49 01 d7             	add    %rdx,%r15
   27fce:	4e 8d 2c f4          	lea    (%rsp,%r14,8),%r13
   27fd2:	49 81 c5 b0 00 00 00 	add    $0xb0,%r13
   27fd9:	4f 8d 3c f7          	lea    (%r15,%r14,8),%r15
   27fdd:	f2 43 0f 10 74 cd 00 	movsd  0x0(%r13,%r9,8),%xmm6
   27fe4:	f2 0f 59 f5          	mulsd  %xmm5,%xmm6
   27fe8:	f2 43 0f 58 34 cf    	addsd  (%r15,%r9,8),%xmm6
   27fee:	f2 43 0f 11 34 cf    	movsd  %xmm6,(%r15,%r9,8)
   27ff4:	4f 8d 0c 76          	lea    (%r14,%r14,2),%r9
   27ff8:	66 0f 28 f5          	movapd %xmm5,%xmm6
   27ffc:	66 0f 14 f5          	unpcklpd %xmm5,%xmm6
   28000:	66 0f 59 34 24       	mulpd  (%rsp),%xmm6
   28005:	66 42 0f 10 3c cf    	movupd (%rdi,%r9,8),%xmm7
   2800b:	66 0f 58 fe          	addpd  %xmm6,%xmm7
   2800f:	66 42 0f 11 3c cf    	movupd %xmm7,(%rdi,%r9,8)
   28015:	f2 0f 59 e8          	mulsd  %xmm0,%xmm5
   28019:	f2 42 0f 58 6c cf 10 	addsd  0x10(%rdi,%r9,8),%xmm5
   28020:	f2 42 0f 11 6c cf 10 	movsd  %xmm5,0x10(%rdi,%r9,8)
   28027:	49 ff c6             	inc    %r14
   2802a:	49 83 c2 58          	add    $0x58,%r10
   2802e:	49 83 c4 08          	add    $0x8,%r12
   28032:	48 ff ce             	dec    %rsi
   28035:	49 83 fe 0a          	cmp    $0xa,%r14
   28039:	0f 84 d1 fe ff ff    	je     27f10 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x680>
   2803f:	f2 42 0f 10 ac f4 b0 	movsd  0xb0(%rsp,%r14,8),%xmm5
   28046:	00 00 00 
   28049:	f2 0f 59 ec          	mulsd  %xmm4,%xmm5
   2804d:	49 83 fe 09          	cmp    $0x9,%r14
   28051:	0f 84 69 ff ff ff    	je     27fc0 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x730>
   28057:	49 89 f7             	mov    %rsi,%r15
   2805a:	49 83 e7 fe          	and    $0xfffffffffffffffe,%r15
   2805e:	bd 0a 00 00 00       	mov    $0xa,%ebp
   28063:	4c 29 f5             	sub    %r14,%rbp
   28066:	45 31 c9             	xor    %r9d,%r9d
   28069:	0f 1f 80 00 00 00 00 	nopl   0x0(%rax)
   28070:	f2 43 0f 10 74 cc f8 	movsd  -0x8(%r12,%r9,8),%xmm6
   28077:	f2 0f 59 f5          	mulsd  %xmm5,%xmm6
   2807b:	f2 43 0f 58 74 ca f8 	addsd  -0x8(%r10,%r9,8),%xmm6
   28082:	f2 43 0f 11 74 ca f8 	movsd  %xmm6,-0x8(%r10,%r9,8)
   28089:	f2 43 0f 10 34 cc    	movsd  (%r12,%r9,8),%xmm6
   2808f:	f2 0f 59 f5          	mulsd  %xmm5,%xmm6
   28093:	f2 43 0f 58 34 ca    	addsd  (%r10,%r9,8),%xmm6
   28099:	f2 43 0f 11 34 ca    	movsd  %xmm6,(%r10,%r9,8)
   2809f:	49 83 c1 02          	add    $0x2,%r9
   280a3:	4d 39 cf             	cmp    %r9,%r15
   280a6:	75 c8                	jne    28070 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x7e0>
   280a8:	40 f6 c5 01          	test   $0x1,%bpl
   280ac:	0f 85 11 ff ff ff    	jne    27fc3 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x733>
   280b2:	e9 3d ff ff ff       	jmp    27ff4 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x764>
   280b7:	66 0f 1f 84 00 00 00 	nopw   0x0(%rax,%rax,1)
   280be:	00 00 
   280c0:	48 8b 44 24 50       	mov    0x50(%rsp),%rax
   280c5:	4c 8d 04 c0          	lea    (%rax,%rax,8),%r8
   280c9:	66 0f 28 d9          	movapd %xmm1,%xmm3
   280cd:	f2 0f 59 9c 24 20 01 	mulsd  0x120(%rsp),%xmm3
   280d4:	00 00 
   280d6:	66 0f 57 e4          	xorpd  %xmm4,%xmm4
   280da:	66 0f 2e e3          	ucomisd %xmm3,%xmm4
   280de:	0f 83 cc 01 00 00    	jae    282b0 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0xa20>
   280e4:	48 8b 84 24 c8 01 00 	mov    0x1c8(%rsp),%rax
   280eb:	00 
   280ec:	4e 8d 14 00          	lea    (%rax,%r8,1),%r10
   280f0:	49 c1 e2 03          	shl    $0x3,%r10
   280f4:	4c 69 5c 24 50 00 e1 	imul   $0xe100,0x50(%rsp),%r11
   280fb:	00 00 
   280fd:	4c 03 9c 24 c0 01 00 	add    0x1c0(%rsp),%r11
   28104:	00 
   28105:	31 db                	xor    %ebx,%ebx
   28107:	45 31 e4             	xor    %r12d,%r12d
   2810a:	eb 1c                	jmp    28128 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x898>
   2810c:	0f 1f 40 00          	nopl   0x0(%rax)
   28110:	48 83 c3 08          	add    $0x8,%rbx
   28114:	49 ff c4             	inc    %r12
   28117:	49 81 c3 20 03 00 00 	add    $0x320,%r11
   2811e:	48 83 fb 40          	cmp    $0x40,%rbx
   28122:	0f 84 88 01 00 00    	je     282b0 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0xa20>
   28128:	f2 0f 10 a4 1c 30 02 	movsd  0x230(%rsp,%rbx,1),%xmm4
   2812f:	00 00 
   28131:	f2 0f 59 e3          	mulsd  %xmm3,%xmm4
   28135:	f2 0f 5e 64 24 60    	divsd  0x60(%rsp),%xmm4
   2813b:	66 0f 57 ed          	xorpd  %xmm5,%xmm5
   2813f:	66 0f 2e ec          	ucomisd %xmm4,%xmm5
   28143:	73 cb                	jae    28110 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x880>
   28145:	4b 8d 3c 14          	lea    (%r12,%r10,1),%rdi
   28149:	4c 6b f7 64          	imul   $0x64,%rdi,%r14
   2814d:	49 81 fe 1c 51 01 00 	cmp    $0x1511c,%r14
   28154:	0f 87 4e 0f 00 00    	ja     290a8 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1818>
   2815a:	48 8d 04 3f          	lea    (%rdi,%rdi,1),%rax
   2815e:	48 c1 e7 05          	shl    $0x5,%rdi
   28162:	48 29 c7             	sub    %rax,%rdi
   28165:	48 8d 77 1e          	lea    0x1e(%rdi),%rsi
   28169:	48 8b 94 24 88 00 00 	mov    0x88(%rsp),%rdx
   28170:	00 
   28171:	48 83 ff e1          	cmp    $0xffffffffffffffe1,%rdi
   28175:	0f 87 42 0f 00 00    	ja     290bd <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x182d>
   2817b:	48 39 d6             	cmp    %rdx,%rsi
   2817e:	0f 87 39 0f 00 00    	ja     290bd <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x182d>
   28184:	48 8b 44 24 18       	mov    0x18(%rsp),%rax
   28189:	4a 8d 14 f0          	lea    (%rax,%r14,8),%rdx
   2818d:	48 c1 e7 03          	shl    $0x3,%rdi
   28191:	48 03 bc 24 80 00 00 	add    0x80(%rsp),%rdi
   28198:	00 
   28199:	41 bd 0a 00 00 00    	mov    $0xa,%r13d
   2819f:	48 8d 84 24 b8 00 00 	lea    0xb8(%rsp),%rax
   281a6:	00 
   281a7:	4d 89 de             	mov    %r11,%r14
   281aa:	45 31 c9             	xor    %r9d,%r9d
   281ad:	eb 78                	jmp    28227 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x997>
   281af:	90                   	nop
   281b0:	31 ed                	xor    %ebp,%ebp
   281b2:	4b 8d 34 89          	lea    (%r9,%r9,4),%rsi
   281b6:	48 c1 e6 04          	shl    $0x4,%rsi
   281ba:	48 01 d6             	add    %rdx,%rsi
   281bd:	4e 8d 3c cc          	lea    (%rsp,%r9,8),%r15
   281c1:	49 81 c7 b0 00 00 00 	add    $0xb0,%r15
   281c8:	4a 8d 34 ce          	lea    (%rsi,%r9,8),%rsi
   281cc:	f2 41 0f 10 34 ef    	movsd  (%r15,%rbp,8),%xmm6
   281d2:	f2 0f 59 f5          	mulsd  %xmm5,%xmm6
   281d6:	f2 0f 58 34 ee       	addsd  (%rsi,%rbp,8),%xmm6
   281db:	f2 0f 11 34 ee       	movsd  %xmm6,(%rsi,%rbp,8)
   281e0:	4b 8d 34 49          	lea    (%r9,%r9,2),%rsi
   281e4:	66 0f 28 f5          	movapd %xmm5,%xmm6
   281e8:	66 0f 14 f5          	unpcklpd %xmm5,%xmm6
   281ec:	66 0f 59 34 24       	mulpd  (%rsp),%xmm6
   281f1:	66 0f 10 3c f7       	movupd (%rdi,%rsi,8),%xmm7
   281f6:	66 0f 58 fe          	addpd  %xmm6,%xmm7
   281fa:	66 0f 11 3c f7       	movupd %xmm7,(%rdi,%rsi,8)
   281ff:	f2 0f 59 e8          	mulsd  %xmm0,%xmm5
   28203:	f2 0f 58 6c f7 10    	addsd  0x10(%rdi,%rsi,8),%xmm5
   28209:	f2 0f 11 6c f7 10    	movsd  %xmm5,0x10(%rdi,%rsi,8)
   2820f:	49 ff c1             	inc    %r9
   28212:	49 83 c6 58          	add    $0x58,%r14
   28216:	48 83 c0 08          	add    $0x8,%rax
   2821a:	49 ff cd             	dec    %r13
   2821d:	49 83 f9 0a          	cmp    $0xa,%r9
   28221:	0f 84 e9 fe ff ff    	je     28110 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x880>
   28227:	f2 42 0f 10 ac cc b0 	movsd  0xb0(%rsp,%r9,8),%xmm5
   2822e:	00 00 00 
   28231:	f2 0f 59 ec          	mulsd  %xmm4,%xmm5
   28235:	49 83 f9 09          	cmp    $0x9,%r9
   28239:	0f 84 71 ff ff ff    	je     281b0 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x920>
   2823f:	4c 89 ee             	mov    %r13,%rsi
   28242:	48 83 e6 fe          	and    $0xfffffffffffffffe,%rsi
   28246:	41 bf 0a 00 00 00    	mov    $0xa,%r15d
   2824c:	4d 29 cf             	sub    %r9,%r15
   2824f:	31 ed                	xor    %ebp,%ebp
   28251:	66 66 66 66 66 66 2e 	data16 data16 data16 data16 data16 cs nopw 0x0(%rax,%rax,1)
   28258:	0f 1f 84 00 00 00 00 
   2825f:	00 
   28260:	f2 0f 10 74 e8 f8    	movsd  -0x8(%rax,%rbp,8),%xmm6
   28266:	f2 0f 59 f5          	mulsd  %xmm5,%xmm6
   2826a:	f2 41 0f 58 74 ee f8 	addsd  -0x8(%r14,%rbp,8),%xmm6
   28271:	f2 41 0f 11 74 ee f8 	movsd  %xmm6,-0x8(%r14,%rbp,8)
   28278:	f2 0f 10 34 e8       	movsd  (%rax,%rbp,8),%xmm6
   2827d:	f2 0f 59 f5          	mulsd  %xmm5,%xmm6
   28281:	f2 41 0f 58 34 ee    	addsd  (%r14,%rbp,8),%xmm6
   28287:	f2 41 0f 11 34 ee    	movsd  %xmm6,(%r14,%rbp,8)
   2828d:	48 83 c5 02          	add    $0x2,%rbp
   28291:	48 39 ee             	cmp    %rbp,%rsi
   28294:	75 ca                	jne    28260 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x9d0>
   28296:	41 f6 c7 01          	test   $0x1,%r15b
   2829a:	0f 85 12 ff ff ff    	jne    281b2 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x922>
   282a0:	e9 3b ff ff ff       	jmp    281e0 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x950>
   282a5:	66 66 2e 0f 1f 84 00 	data16 cs nopw 0x0(%rax,%rax,1)
   282ac:	00 00 00 00 
   282b0:	f2 0f 59 94 24 18 01 	mulsd  0x118(%rsp),%xmm2
   282b7:	00 00 
   282b9:	66 0f 57 db          	xorpd  %xmm3,%xmm3
   282bd:	66 0f 2e da          	ucomisd %xmm2,%xmm3
   282c1:	48 8d ac 24 30 01 00 	lea    0x130(%rsp),%rbp
   282c8:	00 
   282c9:	4c 8b 6c 24 20       	mov    0x20(%rsp),%r13
   282ce:	0f 83 dc 01 00 00    	jae    284b0 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0xc20>
   282d4:	48 03 8c 24 b8 01 00 	add    0x1b8(%rsp),%rcx
   282db:	00 
   282dc:	48 c1 e1 03          	shl    $0x3,%rcx
   282e0:	4c 69 8c 24 90 00 00 	imul   $0xe100,0x90(%rsp),%r9
   282e7:	00 00 e1 00 00 
   282ec:	4c 03 8c 24 b0 01 00 	add    0x1b0(%rsp),%r9
   282f3:	00 
   282f4:	45 31 d2             	xor    %r10d,%r10d
   282f7:	45 31 db             	xor    %r11d,%r11d
   282fa:	eb 1c                	jmp    28318 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0xa88>
   282fc:	0f 1f 40 00          	nopl   0x0(%rax)
   28300:	49 83 c2 08          	add    $0x8,%r10
   28304:	49 ff c3             	inc    %r11
   28307:	49 81 c1 20 03 00 00 	add    $0x320,%r9
   2830e:	49 83 fa 40          	cmp    $0x40,%r10
   28312:	0f 84 98 01 00 00    	je     284b0 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0xc20>
   28318:	f2 42 0f 10 9c 14 30 	movsd  0x230(%rsp,%r10,1),%xmm3
   2831f:	02 00 00 
   28322:	f2 0f 59 da          	mulsd  %xmm2,%xmm3
   28326:	f2 0f 5e 5c 24 60    	divsd  0x60(%rsp),%xmm3
   2832c:	66 0f 57 e4          	xorpd  %xmm4,%xmm4
   28330:	66 0f 2e e3          	ucomisd %xmm3,%xmm4
   28334:	73 ca                	jae    28300 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0xa70>
   28336:	49 8d 3c 0b          	lea    (%r11,%rcx,1),%rdi
   2833a:	4c 6b f7 64          	imul   $0x64,%rdi,%r14
   2833e:	49 81 fe 1c 51 01 00 	cmp    $0x1511c,%r14
   28345:	0f 87 5d 0d 00 00    	ja     290a8 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1818>
   2834b:	48 8d 04 3f          	lea    (%rdi,%rdi,1),%rax
   2834f:	48 c1 e7 05          	shl    $0x5,%rdi
   28353:	48 29 c7             	sub    %rax,%rdi
   28356:	48 8d 77 1e          	lea    0x1e(%rdi),%rsi
   2835a:	48 8b 94 24 88 00 00 	mov    0x88(%rsp),%rdx
   28361:	00 
   28362:	48 83 ff e1          	cmp    $0xffffffffffffffe1,%rdi
   28366:	0f 87 51 0d 00 00    	ja     290bd <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x182d>
   2836c:	48 39 d6             	cmp    %rdx,%rsi
   2836f:	0f 87 48 0d 00 00    	ja     290bd <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x182d>
   28375:	48 8b 44 24 18       	mov    0x18(%rsp),%rax
   2837a:	4a 8d 14 f0          	lea    (%rax,%r14,8),%rdx
   2837e:	48 c1 e7 03          	shl    $0x3,%rdi
   28382:	48 03 bc 24 80 00 00 	add    0x80(%rsp),%rdi
   28389:	00 
   2838a:	be 0a 00 00 00       	mov    $0xa,%esi
   2838f:	48 8d 84 24 b8 00 00 	lea    0xb8(%rsp),%rax
   28396:	00 
   28397:	4c 89 cb             	mov    %r9,%rbx
   2839a:	45 31 f6             	xor    %r14d,%r14d
   2839d:	e9 92 00 00 00       	jmp    28434 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0xba4>
   283a2:	66 66 66 66 66 2e 0f 	data16 data16 data16 data16 cs nopw 0x0(%rax,%rax,1)
   283a9:	1f 84 00 00 00 00 00 
   283b0:	45 31 e4             	xor    %r12d,%r12d
   283b3:	4f 8d 3c b6          	lea    (%r14,%r14,4),%r15
   283b7:	49 c1 e7 04          	shl    $0x4,%r15
   283bb:	49 01 d7             	add    %rdx,%r15
   283be:	4e 8d 2c f4          	lea    (%rsp,%r14,8),%r13
   283c2:	49 81 c5 b0 00 00 00 	add    $0xb0,%r13
   283c9:	4f 8d 3c f7          	lea    (%r15,%r14,8),%r15
   283cd:	f2 43 0f 10 6c e5 00 	movsd  0x0(%r13,%r12,8),%xmm5
   283d4:	4c 8b 6c 24 20       	mov    0x20(%rsp),%r13
   283d9:	f2 0f 59 ec          	mulsd  %xmm4,%xmm5
   283dd:	f2 43 0f 58 2c e7    	addsd  (%r15,%r12,8),%xmm5
   283e3:	f2 43 0f 11 2c e7    	movsd  %xmm5,(%r15,%r12,8)
   283e9:	4f 8d 3c 76          	lea    (%r14,%r14,2),%r15
   283ed:	66 0f 28 ec          	movapd %xmm4,%xmm5
   283f1:	66 0f 14 ec          	unpcklpd %xmm4,%xmm5
   283f5:	66 0f 59 2c 24       	mulpd  (%rsp),%xmm5
   283fa:	66 42 0f 10 34 ff    	movupd (%rdi,%r15,8),%xmm6
   28400:	66 0f 58 f5          	addpd  %xmm5,%xmm6
   28404:	66 42 0f 11 34 ff    	movupd %xmm6,(%rdi,%r15,8)
   2840a:	f2 0f 59 e0          	mulsd  %xmm0,%xmm4
   2840e:	f2 42 0f 58 64 ff 10 	addsd  0x10(%rdi,%r15,8),%xmm4
   28415:	f2 42 0f 11 64 ff 10 	movsd  %xmm4,0x10(%rdi,%r15,8)
   2841c:	49 ff c6             	inc    %r14
   2841f:	48 83 c3 58          	add    $0x58,%rbx
   28423:	48 83 c0 08          	add    $0x8,%rax
   28427:	48 ff ce             	dec    %rsi
   2842a:	49 83 fe 0a          	cmp    $0xa,%r14
   2842e:	0f 84 cc fe ff ff    	je     28300 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0xa70>
   28434:	f2 42 0f 10 a4 f4 b0 	movsd  0xb0(%rsp,%r14,8),%xmm4
   2843b:	00 00 00 
   2843e:	f2 0f 59 e3          	mulsd  %xmm3,%xmm4
   28442:	49 83 fe 09          	cmp    $0x9,%r14
   28446:	0f 84 64 ff ff ff    	je     283b0 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0xb20>
   2844c:	49 89 f7             	mov    %rsi,%r15
   2844f:	49 83 e7 fe          	and    $0xfffffffffffffffe,%r15
   28453:	bd 0a 00 00 00       	mov    $0xa,%ebp
   28458:	4c 29 f5             	sub    %r14,%rbp
   2845b:	45 31 e4             	xor    %r12d,%r12d
   2845e:	66 90                	xchg   %ax,%ax
   28460:	f2 42 0f 10 6c e0 f8 	movsd  -0x8(%rax,%r12,8),%xmm5
   28467:	f2 0f 59 ec          	mulsd  %xmm4,%xmm5
   2846b:	f2 42 0f 58 6c e3 f8 	addsd  -0x8(%rbx,%r12,8),%xmm5
   28472:	f2 42 0f 11 6c e3 f8 	movsd  %xmm5,-0x8(%rbx,%r12,8)
   28479:	f2 42 0f 10 2c e0    	movsd  (%rax,%r12,8),%xmm5
   2847f:	f2 0f 59 ec          	mulsd  %xmm4,%xmm5
   28483:	f2 42 0f 58 2c e3    	addsd  (%rbx,%r12,8),%xmm5
   28489:	f2 42 0f 11 2c e3    	movsd  %xmm5,(%rbx,%r12,8)
   2848f:	49 83 c4 02          	add    $0x2,%r12
   28493:	4d 39 e7             	cmp    %r12,%r15
   28496:	75 c8                	jne    28460 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0xbd0>
   28498:	40 f6 c5 01          	test   $0x1,%bpl
   2849c:	48 8d ac 24 30 01 00 	lea    0x130(%rsp),%rbp
   284a3:	00 
   284a4:	0f 85 09 ff ff ff    	jne    283b3 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0xb23>
   284aa:	e9 3a ff ff ff       	jmp    283e9 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0xb59>
   284af:	90                   	nop
   284b0:	f2 0f 59 8c 24 18 01 	mulsd  0x118(%rsp),%xmm1
   284b7:	00 00 
   284b9:	66 0f 57 d2          	xorpd  %xmm2,%xmm2
   284bd:	66 0f 2e d1          	ucomisd %xmm1,%xmm2
   284c1:	0f 83 e9 f5 ff ff    	jae    27ab0 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x220>
   284c7:	4c 03 84 24 b8 01 00 	add    0x1b8(%rsp),%r8
   284ce:	00 
   284cf:	49 c1 e0 03          	shl    $0x3,%r8
   284d3:	48 69 44 24 50 00 e1 	imul   $0xe100,0x50(%rsp),%rax
   284da:	00 00 
   284dc:	48 03 84 24 b0 01 00 	add    0x1b0(%rsp),%rax
   284e3:	00 
   284e4:	31 c9                	xor    %ecx,%ecx
   284e6:	45 31 c9             	xor    %r9d,%r9d
   284e9:	eb 1c                	jmp    28507 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0xc77>
   284eb:	0f 1f 44 00 00       	nopl   0x0(%rax,%rax,1)
   284f0:	48 83 c1 08          	add    $0x8,%rcx
   284f4:	49 ff c1             	inc    %r9
   284f7:	48 05 20 03 00 00    	add    $0x320,%rax
   284fd:	48 83 f9 40          	cmp    $0x40,%rcx
   28501:	0f 84 a9 f5 ff ff    	je     27ab0 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x220>
   28507:	f2 0f 10 94 0c 30 02 	movsd  0x230(%rsp,%rcx,1),%xmm2
   2850e:	00 00 
   28510:	f2 0f 59 d1          	mulsd  %xmm1,%xmm2
   28514:	f2 0f 5e 54 24 60    	divsd  0x60(%rsp),%xmm2
   2851a:	66 0f 57 db          	xorpd  %xmm3,%xmm3
   2851e:	66 0f 2e da          	ucomisd %xmm2,%xmm3
   28522:	73 cc                	jae    284f0 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0xc60>
   28524:	4b 8d 3c 01          	lea    (%r9,%r8,1),%rdi
   28528:	4c 6b f7 64          	imul   $0x64,%rdi,%r14
   2852c:	49 81 fe 1c 51 01 00 	cmp    $0x1511c,%r14
   28533:	0f 87 6f 0b 00 00    	ja     290a8 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1818>
   28539:	48 8d 14 3f          	lea    (%rdi,%rdi,1),%rdx
   2853d:	48 c1 e7 05          	shl    $0x5,%rdi
   28541:	48 29 d7             	sub    %rdx,%rdi
   28544:	48 8d 77 1e          	lea    0x1e(%rdi),%rsi
   28548:	48 8b 94 24 88 00 00 	mov    0x88(%rsp),%rdx
   2854f:	00 
   28550:	48 83 ff e1          	cmp    $0xffffffffffffffe1,%rdi
   28554:	0f 87 63 0b 00 00    	ja     290bd <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x182d>
   2855a:	48 39 d6             	cmp    %rdx,%rsi
   2855d:	0f 87 5a 0b 00 00    	ja     290bd <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x182d>
   28563:	48 8b 54 24 18       	mov    0x18(%rsp),%rdx
   28568:	4a 8d 14 f2          	lea    (%rdx,%r14,8),%rdx
   2856c:	48 c1 e7 03          	shl    $0x3,%rdi
   28570:	48 03 bc 24 80 00 00 	add    0x80(%rsp),%rdi
   28577:	00 
   28578:	be 0a 00 00 00       	mov    $0xa,%esi
   2857d:	4c 8d 94 24 b8 00 00 	lea    0xb8(%rsp),%r10
   28584:	00 
   28585:	49 89 c3             	mov    %rax,%r11
   28588:	31 db                	xor    %ebx,%ebx
   2858a:	e9 7f 00 00 00       	jmp    2860e <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0xd7e>
   2858f:	90                   	nop
   28590:	45 31 f6             	xor    %r14d,%r14d
   28593:	4c 8d 3c 9b          	lea    (%rbx,%rbx,4),%r15
   28597:	49 c1 e7 04          	shl    $0x4,%r15
   2859b:	49 01 d7             	add    %rdx,%r15
   2859e:	4c 8d 24 dc          	lea    (%rsp,%rbx,8),%r12
   285a2:	49 81 c4 b0 00 00 00 	add    $0xb0,%r12
   285a9:	4d 8d 3c df          	lea    (%r15,%rbx,8),%r15
   285ad:	f2 43 0f 10 24 f4    	movsd  (%r12,%r14,8),%xmm4
   285b3:	f2 0f 59 e3          	mulsd  %xmm3,%xmm4
   285b7:	f2 43 0f 58 24 f7    	addsd  (%r15,%r14,8),%xmm4
   285bd:	f2 43 0f 11 24 f7    	movsd  %xmm4,(%r15,%r14,8)
   285c3:	4c 8d 34 5b          	lea    (%rbx,%rbx,2),%r14
   285c7:	66 0f 28 e3          	movapd %xmm3,%xmm4
   285cb:	66 0f 14 e3          	unpcklpd %xmm3,%xmm4
   285cf:	66 0f 59 24 24       	mulpd  (%rsp),%xmm4
   285d4:	66 42 0f 10 2c f7    	movupd (%rdi,%r14,8),%xmm5
   285da:	66 0f 58 ec          	addpd  %xmm4,%xmm5
   285de:	66 42 0f 11 2c f7    	movupd %xmm5,(%rdi,%r14,8)
   285e4:	f2 0f 59 d8          	mulsd  %xmm0,%xmm3
   285e8:	f2 42 0f 58 5c f7 10 	addsd  0x10(%rdi,%r14,8),%xmm3
   285ef:	f2 42 0f 11 5c f7 10 	movsd  %xmm3,0x10(%rdi,%r14,8)
   285f6:	48 ff c3             	inc    %rbx
   285f9:	49 83 c3 58          	add    $0x58,%r11
   285fd:	49 83 c2 08          	add    $0x8,%r10
   28601:	48 ff ce             	dec    %rsi
   28604:	48 83 fb 0a          	cmp    $0xa,%rbx
   28608:	0f 84 e2 fe ff ff    	je     284f0 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0xc60>
   2860e:	f2 0f 10 9c dc b0 00 	movsd  0xb0(%rsp,%rbx,8),%xmm3
   28615:	00 00 
   28617:	f2 0f 59 da          	mulsd  %xmm2,%xmm3
   2861b:	48 83 fb 09          	cmp    $0x9,%rbx
   2861f:	0f 84 6b ff ff ff    	je     28590 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0xd00>
   28625:	49 89 f7             	mov    %rsi,%r15
   28628:	49 83 e7 fe          	and    $0xfffffffffffffffe,%r15
   2862c:	41 bc 0a 00 00 00    	mov    $0xa,%r12d
   28632:	49 29 dc             	sub    %rbx,%r12
   28635:	45 31 f6             	xor    %r14d,%r14d
   28638:	0f 1f 84 00 00 00 00 	nopl   0x0(%rax,%rax,1)
   2863f:	00 
   28640:	f2 43 0f 10 64 f2 f8 	movsd  -0x8(%r10,%r14,8),%xmm4
   28647:	f2 0f 59 e3          	mulsd  %xmm3,%xmm4
   2864b:	f2 43 0f 58 64 f3 f8 	addsd  -0x8(%r11,%r14,8),%xmm4
   28652:	f2 43 0f 11 64 f3 f8 	movsd  %xmm4,-0x8(%r11,%r14,8)
   28659:	f2 43 0f 10 24 f2    	movsd  (%r10,%r14,8),%xmm4
   2865f:	f2 0f 59 e3          	mulsd  %xmm3,%xmm4
   28663:	f2 43 0f 58 24 f3    	addsd  (%r11,%r14,8),%xmm4
   28669:	f2 43 0f 11 24 f3    	movsd  %xmm4,(%r11,%r14,8)
   2866f:	49 83 c6 02          	add    $0x2,%r14
   28673:	4d 39 f7             	cmp    %r14,%r15
   28676:	75 c8                	jne    28640 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0xdb0>
   28678:	41 f6 c4 01          	test   $0x1,%r12b
   2867c:	0f 85 11 ff ff ff    	jne    28593 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0xd03>
   28682:	e9 3c ff ff ff       	jmp    285c3 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0xd33>
   28687:	48 8b 84 24 10 02 00 	mov    0x210(%rsp),%rax
   2868e:	00 
   2868f:	49 89 c5             	mov    %rax,%r13
   28692:	48 8b 94 24 70 01 00 	mov    0x170(%rsp),%rdx
   28699:	00 
   2869a:	48 39 d0             	cmp    %rdx,%rax
   2869d:	0f 85 72 f3 ff ff    	jne    27a15 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x185>
   286a3:	48 8d bc 24 30 01 00 	lea    0x130(%rsp),%rdi
   286aa:	00 
   286ab:	be 40 65 00 00       	mov    $0x6540,%esi
   286b0:	e8 3b df ff ff       	call   265f0 <<f64 as alloc::vec::spec_from_elem::SpecFromElem>::from_elem::<alloc::alloc::Global>>
   286b5:	b8 80 51 01 00       	mov    $0x15180,%eax
   286ba:	31 c9                	xor    %ecx,%ecx
   286bc:	31 d2                	xor    %edx,%edx
   286be:	48 8b 74 24 18       	mov    0x18(%rsp),%rsi
   286c3:	31 ff                	xor    %edi,%edi
   286c5:	4c 8d 04 ff          	lea    (%rdi,%rdi,8),%r8
   286c9:	4c 89 84 24 f0 01 00 	mov    %r8,0x1f0(%rsp)
   286d0:	00 
   286d1:	48 ff c7             	inc    %rdi
   286d4:	48 89 bc 24 18 01 00 	mov    %rdi,0x118(%rsp)
   286db:	00 
   286dc:	48 89 8c 24 e0 01 00 	mov    %rcx,0x1e0(%rsp)
   286e3:	00 
   286e4:	48 89 94 24 d0 01 00 	mov    %rdx,0x1d0(%rsp)
   286eb:	00 
   286ec:	48 89 b4 24 20 01 00 	mov    %rsi,0x120(%rsp)
   286f3:	00 
   286f4:	48 89 84 24 78 01 00 	mov    %rax,0x178(%rsp)
   286fb:	00 
   286fc:	31 ff                	xor    %edi,%edi
   286fe:	eb 4d                	jmp    2874d <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0xebd>
   28700:	48 8b 84 24 00 02 00 	mov    0x200(%rsp),%rax
   28707:	00 
   28708:	48 05 e0 fc ff ff    	add    $0xfffffffffffffce0,%rax
   2870e:	48 8b b4 24 50 01 00 	mov    0x150(%rsp),%rsi
   28715:	00 
   28716:	48 81 c6 00 19 00 00 	add    $0x1900,%rsi
   2871d:	48 8b 94 24 80 01 00 	mov    0x180(%rsp),%rdx
   28724:	00 
   28725:	48 81 c2 20 03 00 00 	add    $0x320,%rdx
   2872c:	48 8b 8c 24 90 01 00 	mov    0x190(%rsp),%rcx
   28733:	00 
   28734:	48 81 c1 f0 00 00 00 	add    $0xf0,%rcx
   2873b:	48 8b bc 24 28 01 00 	mov    0x128(%rsp),%rdi
   28742:	00 
   28743:	48 83 ff 09          	cmp    $0x9,%rdi
   28747:	0f 84 84 08 00 00    	je     28fd1 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1741>
   2874d:	49 89 f8             	mov    %rdi,%r8
   28750:	48 ff c7             	inc    %rdi
   28753:	48 89 bc 24 28 01 00 	mov    %rdi,0x128(%rsp)
   2875a:	00 
   2875b:	4c 03 84 24 f0 01 00 	add    0x1f0(%rsp),%r8
   28762:	00 
   28763:	49 c1 e0 03          	shl    $0x3,%r8
   28767:	4c 89 44 24 70       	mov    %r8,0x70(%rsp)
   2876c:	48 89 8c 24 90 01 00 	mov    %rcx,0x190(%rsp)
   28773:	00 
   28774:	49 89 cc             	mov    %rcx,%r12
   28777:	48 89 94 24 80 01 00 	mov    %rdx,0x180(%rsp)
   2877e:	00 
   2877f:	48 89 d3             	mov    %rdx,%rbx
   28782:	48 89 b4 24 50 01 00 	mov    %rsi,0x150(%rsp)
   28789:	00 
   2878a:	48 89 74 24 20       	mov    %rsi,0x20(%rsp)
   2878f:	48 89 84 24 00 02 00 	mov    %rax,0x200(%rsp)
   28796:	00 
   28797:	48 89 44 24 60       	mov    %rax,0x60(%rsp)
   2879c:	31 ed                	xor    %ebp,%ebp
   2879e:	eb 43                	jmp    287e3 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0xf53>
   287a0:	be 20 03 00 00       	mov    $0x320,%esi
   287a5:	ba 08 00 00 00       	mov    $0x8,%edx
   287aa:	4c 89 f7             	mov    %r14,%rdi
   287ad:	ff d3                	call   *%rbx
   287af:	48 83 44 24 60 9c    	addq   $0xffffffffffffff9c,0x60(%rsp)
   287b5:	48 81 44 24 20 20 03 	addq   $0x320,0x20(%rsp)
   287bc:	00 00 
   287be:	48 8b 9c 24 90 00 00 	mov    0x90(%rsp),%rbx
   287c5:	00 
   287c6:	48 83 c3 64          	add    $0x64,%rbx
   287ca:	49 83 c4 1e          	add    $0x1e,%r12
   287ce:	48 8b 84 24 00 01 00 	mov    0x100(%rsp),%rax
   287d5:	00 
   287d6:	48 89 c5             	mov    %rax,%rbp
   287d9:	48 83 f8 08          	cmp    $0x8,%rax
   287dd:	0f 84 1d ff ff ff    	je     28700 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0xe70>
   287e3:	ff 15 2f af 04 00    	call   *0x4af2f(%rip)        # 73718 <_DYNAMIC+0x288>
   287e9:	41 bf 20 03 00 00    	mov    $0x320,%r15d
   287ef:	41 bd 08 00 00 00    	mov    $0x8,%r13d
   287f5:	bf 20 03 00 00       	mov    $0x320,%edi
   287fa:	be 08 00 00 00       	mov    $0x8,%esi
   287ff:	ff 15 53 b0 04 00    	call   *0x4b053(%rip)        # 73858 <_DYNAMIC+0x3c8>
   28805:	48 85 c0             	test   %rax,%rax
   28808:	0f 84 a4 10 00 00    	je     298b2 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x2022>
   2880e:	49 89 c6             	mov    %rax,%r14
   28811:	48 8d 45 01          	lea    0x1(%rbp),%rax
   28815:	48 89 84 24 00 01 00 	mov    %rax,0x100(%rsp)
   2881c:	00 
   2881d:	48 03 6c 24 70       	add    0x70(%rsp),%rbp
   28822:	48 89 6c 24 50       	mov    %rbp,0x50(%rsp)
   28827:	49 8d 46 18          	lea    0x18(%r14),%rax
   2882b:	b9 64 00 00 00       	mov    $0x64,%ecx
   28830:	48 89 9c 24 90 00 00 	mov    %rbx,0x90(%rsp)
   28837:	00 
   28838:	48 89 da             	mov    %rbx,%rdx
   2883b:	48 8b 74 24 60       	mov    0x60(%rsp),%rsi
   28840:	4d 89 f0             	mov    %r14,%r8
   28843:	4c 8b 4c 24 20       	mov    0x20(%rsp),%r9
   28848:	45 31 d2             	xor    %r10d,%r10d
   2884b:	4c 89 e3             	mov    %r12,%rbx
   2884e:	4c 89 34 24          	mov    %r14,(%rsp)
   28852:	66 66 66 66 66 2e 0f 	data16 data16 data16 data16 cs nopw 0x0(%rax,%rax,1)
   28859:	1f 84 00 00 00 00 00 
   28860:	4d 85 d2             	test   %r10,%r10
   28863:	74 1b                	je     28880 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0xff0>
   28865:	4b 8d 3c 12          	lea    (%r10,%r10,1),%rdi
   28869:	48 8d 3c bf          	lea    (%rdi,%rdi,4),%rdi
   2886d:	49 83 fa 04          	cmp    $0x4,%r10
   28871:	73 1d                	jae    28890 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1000>
   28873:	45 31 db             	xor    %r11d,%r11d
   28876:	eb 77                	jmp    288ef <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x105f>
   28878:	0f 1f 84 00 00 00 00 	nopl   0x0(%rax,%rax,1)
   2887f:	00 
   28880:	31 ff                	xor    %edi,%edi
   28882:	e9 a1 00 00 00       	jmp    28928 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1098>
   28887:	66 0f 1f 84 00 00 00 	nopw   0x0(%rax,%rax,1)
   2888e:	00 00 
   28890:	4d 89 d7             	mov    %r10,%r15
   28893:	49 83 e7 fc          	and    $0xfffffffffffffffc,%r15
   28897:	4d 89 cd             	mov    %r9,%r13
   2889a:	45 31 db             	xor    %r11d,%r11d
   2889d:	0f 1f 00             	nopl   (%rax)
   288a0:	f2 41 0f 10 45 00    	movsd  0x0(%r13),%xmm0
   288a6:	f2 42 0f 11 44 d8 e8 	movsd  %xmm0,-0x18(%rax,%r11,8)
   288ad:	f2 41 0f 10 45 50    	movsd  0x50(%r13),%xmm0
   288b3:	f2 42 0f 11 44 d8 f0 	movsd  %xmm0,-0x10(%rax,%r11,8)
   288ba:	f2 41 0f 10 85 a0 00 	movsd  0xa0(%r13),%xmm0
   288c1:	00 00 
   288c3:	f2 42 0f 11 44 d8 f8 	movsd  %xmm0,-0x8(%rax,%r11,8)
   288ca:	f2 41 0f 10 85 f0 00 	movsd  0xf0(%r13),%xmm0
   288d1:	00 00 
   288d3:	f2 42 0f 11 04 d8    	movsd  %xmm0,(%rax,%r11,8)
   288d9:	49 83 c3 04          	add    $0x4,%r11
   288dd:	49 81 c5 40 01 00 00 	add    $0x140,%r13
   288e4:	4d 39 df             	cmp    %r11,%r15
   288e7:	75 b7                	jne    288a0 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1010>
   288e9:	41 f6 c2 03          	test   $0x3,%r10b
   288ed:	74 39                	je     28928 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1098>
   288ef:	45 89 d7             	mov    %r10d,%r15d
   288f2:	41 83 e7 03          	and    $0x3,%r15d
   288f6:	4f 8d 2c d8          	lea    (%r8,%r11,8),%r13
   288fa:	4f 8d 1c 9b          	lea    (%r11,%r11,4),%r11
   288fe:	49 c1 e3 04          	shl    $0x4,%r11
   28902:	4d 01 cb             	add    %r9,%r11
   28905:	45 31 e4             	xor    %r12d,%r12d
   28908:	0f 1f 84 00 00 00 00 	nopl   0x0(%rax,%rax,1)
   2890f:	00 
   28910:	f2 41 0f 10 03       	movsd  (%r11),%xmm0
   28915:	f2 43 0f 11 44 e5 00 	movsd  %xmm0,0x0(%r13,%r12,8)
   2891c:	49 ff c4             	inc    %r12
   2891f:	49 83 c3 50          	add    $0x50,%r11
   28923:	4d 39 e7             	cmp    %r12,%r15
   28926:	75 e8                	jne    28910 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1080>
   28928:	4d 8d 5a 01          	lea    0x1(%r10),%r11
   2892c:	4e 8d 3c 17          	lea    (%rdi,%r10,1),%r15
   28930:	4f 8d 2c fe          	lea    (%r14,%r15,8),%r13
   28934:	48 01 d7             	add    %rdx,%rdi
   28937:	4c 8b 74 24 18       	mov    0x18(%rsp),%r14
   2893c:	4d 8d 24 fe          	lea    (%r14,%rdi,8),%r12
   28940:	31 ed                	xor    %ebp,%ebp
   28942:	66 66 66 66 66 2e 0f 	data16 data16 data16 data16 cs nopw 0x0(%rax,%rax,1)
   28949:	1f 84 00 00 00 00 00 
   28950:	48 39 ee             	cmp    %rbp,%rsi
   28953:	0f 84 76 07 00 00    	je     290cf <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x183f>
   28959:	48 39 e9             	cmp    %rbp,%rcx
   2895c:	0f 84 7e 07 00 00    	je     290e0 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1850>
   28962:	f2 41 0f 10 04 ec    	movsd  (%r12,%rbp,8),%xmm0
   28968:	f2 41 0f 11 44 ed 00 	movsd  %xmm0,0x0(%r13,%rbp,8)
   2896f:	4d 8d 34 2a          	lea    (%r10,%rbp,1),%r14
   28973:	49 ff c6             	inc    %r14
   28976:	48 ff c5             	inc    %rbp
   28979:	49 83 fe 0a          	cmp    $0xa,%r14
   2897d:	75 d1                	jne    28950 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x10c0>
   2897f:	49 83 c1 08          	add    $0x8,%r9
   28983:	48 83 c0 50          	add    $0x50,%rax
   28987:	49 83 c0 50          	add    $0x50,%r8
   2898b:	48 83 c1 f5          	add    $0xfffffffffffffff5,%rcx
   2898f:	48 83 c6 f5          	add    $0xfffffffffffffff5,%rsi
   28993:	48 ff c2             	inc    %rdx
   28996:	4d 89 da             	mov    %r11,%r10
   28999:	49 83 fb 0a          	cmp    $0xa,%r11
   2899d:	4c 8b 34 24          	mov    (%rsp),%r14
   289a1:	0f 85 b9 fe ff ff    	jne    28860 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0xfd0>
   289a7:	49 89 dc             	mov    %rbx,%r12
   289aa:	f2 45 0f 10 16       	movsd  (%r14),%xmm10
   289af:	f2 41 0f 10 7e 58    	movsd  0x58(%r14),%xmm7
   289b5:	66 41 0f 28 c2       	movapd %xmm10,%xmm0
   289ba:	f2 0f 58 c7          	addsd  %xmm7,%xmm0
   289be:	f2 45 0f 10 86 b0 00 	movsd  0xb0(%r14),%xmm8
   289c5:	00 00 
   289c7:	f2 41 0f 58 c0       	addsd  %xmm8,%xmm0
   289cc:	f2 45 0f 10 8e 08 01 	movsd  0x108(%r14),%xmm9
   289d3:	00 00 
   289d5:	f2 41 0f 58 c1       	addsd  %xmm9,%xmm0
   289da:	f2 41 0f 10 8e 60 01 	movsd  0x160(%r14),%xmm1
   289e1:	00 00 
   289e3:	f2 0f 58 c1          	addsd  %xmm1,%xmm0
   289e7:	f2 41 0f 10 96 b8 01 	movsd  0x1b8(%r14),%xmm2
   289ee:	00 00 
   289f0:	f2 0f 58 c2          	addsd  %xmm2,%xmm0
   289f4:	f2 41 0f 10 9e 10 02 	movsd  0x210(%r14),%xmm3
   289fb:	00 00 
   289fd:	f2 0f 58 c3          	addsd  %xmm3,%xmm0
   28a01:	f2 41 0f 10 a6 68 02 	movsd  0x268(%r14),%xmm4
   28a08:	00 00 
   28a0a:	f2 0f 58 c4          	addsd  %xmm4,%xmm0
   28a0e:	f2 41 0f 10 ae c0 02 	movsd  0x2c0(%r14),%xmm5
   28a15:	00 00 
   28a17:	f2 0f 58 c5          	addsd  %xmm5,%xmm0
   28a1b:	f2 41 0f 10 b6 18 03 	movsd  0x318(%r14),%xmm6
   28a22:	00 00 
   28a24:	f2 0f 58 c6          	addsd  %xmm6,%xmm0
   28a28:	f2 44 0f 10 1d 0f 43 	movsd  -0x1bcf1(%rip),%xmm11        # cd40 <GCC_except_table0+0xb4>
   28a2f:	fe ff 
   28a31:	f2 41 0f 5e c3       	divsd  %xmm11,%xmm0
   28a36:	f2 0f 59 05 3a 43 fe 	mulsd  -0x1bcc6(%rip),%xmm0        # cd78 <GCC_except_table0+0xec>
   28a3d:	ff 
   28a3e:	f2 0f 5f 05 12 43 fe 	maxsd  -0x1bcee(%rip),%xmm0        # cd58 <GCC_except_table0+0xcc>
   28a45:	ff 
   28a46:	f2 44 0f 58 d0       	addsd  %xmm0,%xmm10
   28a4b:	f2 45 0f 11 16       	movsd  %xmm10,(%r14)
   28a50:	f2 0f 58 f8          	addsd  %xmm0,%xmm7
   28a54:	f2 41 0f 11 7e 58    	movsd  %xmm7,0x58(%r14)
   28a5a:	f2 44 0f 58 c0       	addsd  %xmm0,%xmm8
   28a5f:	f2 45 0f 11 86 b0 00 	movsd  %xmm8,0xb0(%r14)
   28a66:	00 00 
   28a68:	f2 44 0f 58 c8       	addsd  %xmm0,%xmm9
   28a6d:	f2 45 0f 11 8e 08 01 	movsd  %xmm9,0x108(%r14)
   28a74:	00 00 
   28a76:	f2 41 0f 59 c3       	mulsd  %xmm11,%xmm0
   28a7b:	f2 0f 58 c8          	addsd  %xmm0,%xmm1
   28a7f:	f2 41 0f 11 8e 60 01 	movsd  %xmm1,0x160(%r14)
   28a86:	00 00 
   28a88:	f2 0f 58 d0          	addsd  %xmm0,%xmm2
   28a8c:	f2 41 0f 11 96 b8 01 	movsd  %xmm2,0x1b8(%r14)
   28a93:	00 00 
   28a95:	f2 0f 58 d8          	addsd  %xmm0,%xmm3
   28a99:	f2 41 0f 11 9e 10 02 	movsd  %xmm3,0x210(%r14)
   28aa0:	00 00 
   28aa2:	f2 0f 58 e0          	addsd  %xmm0,%xmm4
   28aa6:	f2 41 0f 11 a6 68 02 	movsd  %xmm4,0x268(%r14)
   28aad:	00 00 
   28aaf:	f2 0f 58 e8          	addsd  %xmm0,%xmm5
   28ab3:	f2 41 0f 11 ae c0 02 	movsd  %xmm5,0x2c0(%r14)
   28aba:	00 00 
   28abc:	f2 0f 58 c6          	addsd  %xmm6,%xmm0
   28ac0:	f2 41 0f 11 86 18 03 	movsd  %xmm0,0x318(%r14)
   28ac7:	00 00 
   28ac9:	48 8b 5c 24 50       	mov    0x50(%rsp),%rbx
   28ace:	48 8d 04 1b          	lea    (%rbx,%rbx,1),%rax
   28ad2:	48 c1 e3 05          	shl    $0x5,%rbx
   28ad6:	48 29 c3             	sub    %rax,%rbx
   28ad9:	48 c7 44 24 48 00 00 	movq   $0x0,0x48(%rsp)
   28ae0:	00 00 
   28ae2:	48 89 9c 24 a8 00 00 	mov    %rbx,0xa8(%rsp)
   28ae9:	00 
   28aea:	48 c7 84 24 c8 00 00 	movq   $0x0,0xc8(%rsp)
   28af1:	00 00 00 00 00 
   28af6:	48 c7 84 24 d0 00 00 	movq   $0xa,0xd0(%rsp)
   28afd:	00 0a 00 00 00 
   28b02:	48 8d 44 24 78       	lea    0x78(%rsp),%rax
   28b07:	48 89 84 24 b0 00 00 	mov    %rax,0xb0(%rsp)
   28b0e:	00 
   28b0f:	48 8d 84 24 a8 00 00 	lea    0xa8(%rsp),%rax
   28b16:	00 
   28b17:	48 89 84 24 b8 00 00 	mov    %rax,0xb8(%rsp)
   28b1e:	00 
   28b1f:	48 8d 44 24 48       	lea    0x48(%rsp),%rax
   28b24:	48 89 84 24 c0 00 00 	mov    %rax,0xc0(%rsp)
   28b2b:	00 
   28b2c:	48 8d 7c 24 28       	lea    0x28(%rsp),%rdi
   28b31:	4c 8d bc 24 b0 00 00 	lea    0xb0(%rsp),%r15
   28b38:	00 
   28b39:	4c 89 fe             	mov    %r15,%rsi
   28b3c:	e8 5f 10 00 00       	call   29ba0 <<alloc::vec::Vec<f64> as alloc::vec::spec_from_iter_nested::SpecFromIterNested<f64, core::iter::adapters::map::Map<core::ops::range::Range<usize>, xdremux_engine::apple_photographic_styles::apple_style_revert_local_data::{closure#3}>>>::from_iter>
   28b41:	48 8b 6c 24 30       	mov    0x30(%rsp),%rbp
   28b46:	4c 8b 44 24 38       	mov    0x38(%rsp),%r8
   28b4b:	ba 64 00 00 00       	mov    $0x64,%edx
   28b50:	4c 89 ff             	mov    %r15,%rdi
   28b53:	4c 89 f6             	mov    %r14,%rsi
   28b56:	48 89 e9             	mov    %rbp,%rcx
   28b59:	e8 32 dd ff ff       	call   26890 <xdremux_engine::apple_photographic_styles::solve_linear_system>
   28b5e:	80 bc 24 b0 00 00 00 	cmpb   $0xff,0xb0(%rsp)
   28b65:	ff 
   28b66:	74 38                	je     28ba0 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1310>
   28b68:	ff 15 aa ab 04 00    	call   *0x4abaa(%rip)        # 73718 <_DYNAMIC+0x288>
   28b6e:	bf 50 00 00 00       	mov    $0x50,%edi
   28b73:	be 08 00 00 00       	mov    $0x8,%esi
   28b78:	ff 15 da ac 04 00    	call   *0x4acda(%rip)        # 73858 <_DYNAMIC+0x3c8>
   28b7e:	48 85 c0             	test   %rax,%rax
   28b81:	0f 84 19 0d 00 00    	je     298a0 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x2010>
   28b87:	49 89 c5             	mov    %rax,%r13
   28b8a:	48 83 c0 50          	add    $0x50,%rax
   28b8e:	41 bf 0a 00 00 00    	mov    $0xa,%r15d
   28b94:	eb 32                	jmp    28bc8 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1338>
   28b96:	66 2e 0f 1f 84 00 00 	cs nopw 0x0(%rax,%rax,1)
   28b9d:	00 00 00 
   28ba0:	4c 8b bc 24 b8 00 00 	mov    0xb8(%rsp),%r15
   28ba7:	00 
   28ba8:	4c 8b ac 24 c0 00 00 	mov    0xc0(%rsp),%r13
   28baf:	00 
   28bb0:	48 8b 84 24 c8 00 00 	mov    0xc8(%rsp),%rax
   28bb7:	00 
   28bb8:	48 85 c0             	test   %rax,%rax
   28bbb:	74 7f                	je     28c3c <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x13ac>
   28bbd:	48 8d 04 c5 00 00 00 	lea    0x0(,%rax,8),%rax
   28bc4:	00 
   28bc5:	4c 01 e8             	add    %r13,%rax
   28bc8:	4c 8d 1d 41 48 fe ff 	lea    -0x1b7bf(%rip),%r11        # d410 <GCC_except_table0+0x784>
   28bcf:	66 0f 28 15 a9 47 fe 	movapd -0x1b857(%rip),%xmm2        # d380 <GCC_except_table0+0x6f4>
   28bd6:	ff 
   28bd7:	48 8b 8c 24 38 01 00 	mov    0x138(%rsp),%rcx
   28bde:	00 
   28bdf:	48 8b b4 24 40 01 00 	mov    0x140(%rsp),%rsi
   28be6:	00 
   28be7:	31 d2                	xor    %edx,%edx
   28be9:	4d 89 e8             	mov    %r13,%r8
   28bec:	0f 1f 40 00          	nopl   0x0(%rax)
   28bf0:	4c 8b 4c 24 48       	mov    0x48(%rsp),%r9
   28bf5:	49 8d 3c 14          	lea    (%r12,%rdx,1),%rdi
   28bf9:	4c 01 cf             	add    %r9,%rdi
   28bfc:	48 39 f7             	cmp    %rsi,%rdi
   28bff:	0f 83 21 0b 00 00    	jae    29726 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1e96>
   28c05:	49 01 d1             	add    %rdx,%r9
   28c08:	45 31 d2             	xor    %r10d,%r10d
   28c0b:	49 83 f9 0c          	cmp    $0xc,%r9
   28c0f:	41 0f 93 c2          	setae  %r10b
   28c13:	f2 43 0f 10 04 d3    	movsd  (%r11,%r10,8),%xmm0
   28c19:	66 0f 28 c8          	movapd %xmm0,%xmm1
   28c1d:	66 0f 57 ca          	xorpd  %xmm2,%xmm1
   28c21:	f2 41 0f 5f 08       	maxsd  (%r8),%xmm1
   28c26:	49 83 c0 08          	add    $0x8,%r8
   28c2a:	f2 0f 5d c1          	minsd  %xmm1,%xmm0
   28c2e:	f2 0f 11 04 f9       	movsd  %xmm0,(%rcx,%rdi,8)
   28c33:	48 83 c2 03          	add    $0x3,%rdx
   28c37:	49 39 c0             	cmp    %rax,%r8
   28c3a:	75 b4                	jne    28bf0 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1360>
   28c3c:	4d 85 ff             	test   %r15,%r15
   28c3f:	74 15                	je     28c56 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x13c6>
   28c41:	49 c1 e7 03          	shl    $0x3,%r15
   28c45:	ba 08 00 00 00       	mov    $0x8,%edx
   28c4a:	4c 89 ef             	mov    %r13,%rdi
   28c4d:	4c 89 fe             	mov    %r15,%rsi
   28c50:	ff 15 8a aa 04 00    	call   *0x4aa8a(%rip)        # 736e0 <_DYNAMIC+0x250>
   28c56:	48 8b 74 24 28       	mov    0x28(%rsp),%rsi
   28c5b:	48 85 f6             	test   %rsi,%rsi
   28c5e:	74 12                	je     28c72 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x13e2>
   28c60:	48 c1 e6 03          	shl    $0x3,%rsi
   28c64:	ba 08 00 00 00       	mov    $0x8,%edx
   28c69:	48 89 ef             	mov    %rbp,%rdi
   28c6c:	ff 15 6e aa 04 00    	call   *0x4aa6e(%rip)        # 736e0 <_DYNAMIC+0x250>
   28c72:	48 c7 44 24 48 01 00 	movq   $0x1,0x48(%rsp)
   28c79:	00 00 
   28c7b:	48 89 9c 24 a8 00 00 	mov    %rbx,0xa8(%rsp)
   28c82:	00 
   28c83:	48 c7 84 24 c8 00 00 	movq   $0x0,0xc8(%rsp)
   28c8a:	00 00 00 00 00 
   28c8f:	48 c7 84 24 d0 00 00 	movq   $0xa,0xd0(%rsp)
   28c96:	00 0a 00 00 00 
   28c9b:	48 8d 44 24 78       	lea    0x78(%rsp),%rax
   28ca0:	48 89 84 24 b0 00 00 	mov    %rax,0xb0(%rsp)
   28ca7:	00 
   28ca8:	48 8d 84 24 a8 00 00 	lea    0xa8(%rsp),%rax
   28caf:	00 
   28cb0:	48 89 84 24 b8 00 00 	mov    %rax,0xb8(%rsp)
   28cb7:	00 
   28cb8:	48 8d 44 24 48       	lea    0x48(%rsp),%rax
   28cbd:	48 89 84 24 c0 00 00 	mov    %rax,0xc0(%rsp)
   28cc4:	00 
   28cc5:	48 8d 7c 24 28       	lea    0x28(%rsp),%rdi
   28cca:	4c 8d bc 24 b0 00 00 	lea    0xb0(%rsp),%r15
   28cd1:	00 
   28cd2:	4c 89 fe             	mov    %r15,%rsi
   28cd5:	e8 c6 0e 00 00       	call   29ba0 <<alloc::vec::Vec<f64> as alloc::vec::spec_from_iter_nested::SpecFromIterNested<f64, core::iter::adapters::map::Map<core::ops::range::Range<usize>, xdremux_engine::apple_photographic_styles::apple_style_revert_local_data::{closure#3}>>>::from_iter>
   28cda:	48 8b 6c 24 30       	mov    0x30(%rsp),%rbp
   28cdf:	4c 8b 44 24 38       	mov    0x38(%rsp),%r8
   28ce4:	ba 64 00 00 00       	mov    $0x64,%edx
   28ce9:	4c 89 ff             	mov    %r15,%rdi
   28cec:	4c 89 f6             	mov    %r14,%rsi
   28cef:	48 89 e9             	mov    %rbp,%rcx
   28cf2:	e8 99 db ff ff       	call   26890 <xdremux_engine::apple_photographic_styles::solve_linear_system>
   28cf7:	80 bc 24 b0 00 00 00 	cmpb   $0xff,0xb0(%rsp)
   28cfe:	ff 
   28cff:	74 2f                	je     28d30 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x14a0>
   28d01:	ff 15 11 aa 04 00    	call   *0x4aa11(%rip)        # 73718 <_DYNAMIC+0x288>
   28d07:	bf 50 00 00 00       	mov    $0x50,%edi
   28d0c:	be 08 00 00 00       	mov    $0x8,%esi
   28d11:	ff 15 41 ab 04 00    	call   *0x4ab41(%rip)        # 73858 <_DYNAMIC+0x3c8>
   28d17:	48 85 c0             	test   %rax,%rax
   28d1a:	0f 84 80 0b 00 00    	je     298a0 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x2010>
   28d20:	49 89 c5             	mov    %rax,%r13
   28d23:	48 83 c0 50          	add    $0x50,%rax
   28d27:	41 bf 0a 00 00 00    	mov    $0xa,%r15d
   28d2d:	eb 29                	jmp    28d58 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x14c8>
   28d2f:	90                   	nop
   28d30:	4c 8b bc 24 b8 00 00 	mov    0xb8(%rsp),%r15
   28d37:	00 
   28d38:	4c 8b ac 24 c0 00 00 	mov    0xc0(%rsp),%r13
   28d3f:	00 
   28d40:	48 8b 84 24 c8 00 00 	mov    0xc8(%rsp),%rax
   28d47:	00 
   28d48:	48 85 c0             	test   %rax,%rax
   28d4b:	74 7f                	je     28dcc <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x153c>
   28d4d:	48 8d 04 c5 00 00 00 	lea    0x0(,%rax,8),%rax
   28d54:	00 
   28d55:	4c 01 e8             	add    %r13,%rax
   28d58:	4c 8d 1d b1 46 fe ff 	lea    -0x1b94f(%rip),%r11        # d410 <GCC_except_table0+0x784>
   28d5f:	66 0f 28 15 19 46 fe 	movapd -0x1b9e7(%rip),%xmm2        # d380 <GCC_except_table0+0x6f4>
   28d66:	ff 
   28d67:	48 8b 8c 24 38 01 00 	mov    0x138(%rsp),%rcx
   28d6e:	00 
   28d6f:	48 8b b4 24 40 01 00 	mov    0x140(%rsp),%rsi
   28d76:	00 
   28d77:	31 d2                	xor    %edx,%edx
   28d79:	4d 89 e8             	mov    %r13,%r8
   28d7c:	0f 1f 40 00          	nopl   0x0(%rax)
   28d80:	4c 8b 4c 24 48       	mov    0x48(%rsp),%r9
   28d85:	49 8d 3c 14          	lea    (%r12,%rdx,1),%rdi
   28d89:	4c 01 cf             	add    %r9,%rdi
   28d8c:	48 39 f7             	cmp    %rsi,%rdi
   28d8f:	0f 83 91 09 00 00    	jae    29726 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1e96>
   28d95:	49 01 d1             	add    %rdx,%r9
   28d98:	45 31 d2             	xor    %r10d,%r10d
   28d9b:	49 83 f9 0c          	cmp    $0xc,%r9
   28d9f:	41 0f 93 c2          	setae  %r10b
   28da3:	f2 43 0f 10 04 d3    	movsd  (%r11,%r10,8),%xmm0
   28da9:	66 0f 28 c8          	movapd %xmm0,%xmm1
   28dad:	66 0f 57 ca          	xorpd  %xmm2,%xmm1
   28db1:	f2 41 0f 5f 08       	maxsd  (%r8),%xmm1
   28db6:	49 83 c0 08          	add    $0x8,%r8
   28dba:	f2 0f 5d c1          	minsd  %xmm1,%xmm0
   28dbe:	f2 0f 11 04 f9       	movsd  %xmm0,(%rcx,%rdi,8)
   28dc3:	48 83 c2 03          	add    $0x3,%rdx
   28dc7:	49 39 c0             	cmp    %rax,%r8
   28dca:	75 b4                	jne    28d80 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x14f0>
   28dcc:	4d 85 ff             	test   %r15,%r15
   28dcf:	74 15                	je     28de6 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1556>
   28dd1:	49 c1 e7 03          	shl    $0x3,%r15
   28dd5:	ba 08 00 00 00       	mov    $0x8,%edx
   28dda:	4c 89 ef             	mov    %r13,%rdi
   28ddd:	4c 89 fe             	mov    %r15,%rsi
   28de0:	ff 15 fa a8 04 00    	call   *0x4a8fa(%rip)        # 736e0 <_DYNAMIC+0x250>
   28de6:	48 8b 74 24 28       	mov    0x28(%rsp),%rsi
   28deb:	48 85 f6             	test   %rsi,%rsi
   28dee:	74 12                	je     28e02 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1572>
   28df0:	48 c1 e6 03          	shl    $0x3,%rsi
   28df4:	ba 08 00 00 00       	mov    $0x8,%edx
   28df9:	48 89 ef             	mov    %rbp,%rdi
   28dfc:	ff 15 de a8 04 00    	call   *0x4a8de(%rip)        # 736e0 <_DYNAMIC+0x250>
   28e02:	48 c7 44 24 48 02 00 	movq   $0x2,0x48(%rsp)
   28e09:	00 00 
   28e0b:	48 89 9c 24 a8 00 00 	mov    %rbx,0xa8(%rsp)
   28e12:	00 
   28e13:	48 c7 84 24 c8 00 00 	movq   $0x0,0xc8(%rsp)
   28e1a:	00 00 00 00 00 
   28e1f:	48 c7 84 24 d0 00 00 	movq   $0xa,0xd0(%rsp)
   28e26:	00 0a 00 00 00 
   28e2b:	48 8d 44 24 78       	lea    0x78(%rsp),%rax
   28e30:	48 89 84 24 b0 00 00 	mov    %rax,0xb0(%rsp)
   28e37:	00 
   28e38:	48 8d 84 24 a8 00 00 	lea    0xa8(%rsp),%rax
   28e3f:	00 
   28e40:	48 89 84 24 b8 00 00 	mov    %rax,0xb8(%rsp)
   28e47:	00 
   28e48:	48 8d 44 24 48       	lea    0x48(%rsp),%rax
   28e4d:	48 89 84 24 c0 00 00 	mov    %rax,0xc0(%rsp)
   28e54:	00 
   28e55:	48 8d 7c 24 28       	lea    0x28(%rsp),%rdi
   28e5a:	4c 8d bc 24 b0 00 00 	lea    0xb0(%rsp),%r15
   28e61:	00 
   28e62:	4c 89 fe             	mov    %r15,%rsi
   28e65:	e8 36 0d 00 00       	call   29ba0 <<alloc::vec::Vec<f64> as alloc::vec::spec_from_iter_nested::SpecFromIterNested<f64, core::iter::adapters::map::Map<core::ops::range::Range<usize>, xdremux_engine::apple_photographic_styles::apple_style_revert_local_data::{closure#3}>>>::from_iter>
   28e6a:	48 8b 6c 24 30       	mov    0x30(%rsp),%rbp
   28e6f:	4c 8b 44 24 38       	mov    0x38(%rsp),%r8
   28e74:	ba 64 00 00 00       	mov    $0x64,%edx
   28e79:	4c 89 ff             	mov    %r15,%rdi
   28e7c:	4c 89 f6             	mov    %r14,%rsi
   28e7f:	48 89 e9             	mov    %rbp,%rcx
   28e82:	e8 09 da ff ff       	call   26890 <xdremux_engine::apple_photographic_styles::solve_linear_system>
   28e87:	80 bc 24 b0 00 00 00 	cmpb   $0xff,0xb0(%rsp)
   28e8e:	ff 
   28e8f:	74 4f                	je     28ee0 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1650>
   28e91:	ff 15 81 a8 04 00    	call   *0x4a881(%rip)        # 73718 <_DYNAMIC+0x288>
   28e97:	bf 50 00 00 00       	mov    $0x50,%edi
   28e9c:	be 08 00 00 00       	mov    $0x8,%esi
   28ea1:	ff 15 b1 a9 04 00    	call   *0x4a9b1(%rip)        # 73858 <_DYNAMIC+0x3c8>
   28ea7:	48 85 c0             	test   %rax,%rax
   28eaa:	0f 84 f0 09 00 00    	je     298a0 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x2010>
   28eb0:	49 89 c5             	mov    %rax,%r13
   28eb3:	48 83 c0 50          	add    $0x50,%rax
   28eb7:	41 bf 0a 00 00 00    	mov    $0xa,%r15d
   28ebd:	48 8b 1d 1c a8 04 00 	mov    0x4a81c(%rip),%rbx        # 736e0 <_DYNAMIC+0x250>
   28ec4:	4c 8d 1d 45 45 fe ff 	lea    -0x1babb(%rip),%r11        # d410 <GCC_except_table0+0x784>
   28ecb:	66 0f 28 15 ad 44 fe 	movapd -0x1bb53(%rip),%xmm2        # d380 <GCC_except_table0+0x6f4>
   28ed2:	ff 
   28ed3:	eb 4d                	jmp    28f22 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1692>
   28ed5:	66 66 2e 0f 1f 84 00 	data16 cs nopw 0x0(%rax,%rax,1)
   28edc:	00 00 00 00 
   28ee0:	4c 8b bc 24 b8 00 00 	mov    0xb8(%rsp),%r15
   28ee7:	00 
   28ee8:	4c 8b ac 24 c0 00 00 	mov    0xc0(%rsp),%r13
   28eef:	00 
   28ef0:	48 8b 84 24 c8 00 00 	mov    0xc8(%rsp),%rax
   28ef7:	00 
   28ef8:	48 85 c0             	test   %rax,%rax
   28efb:	4c 8d 1d 0e 45 fe ff 	lea    -0x1baf2(%rip),%r11        # d410 <GCC_except_table0+0x784>
   28f02:	66 0f 28 15 76 44 fe 	movapd -0x1bb8a(%rip),%xmm2        # d380 <GCC_except_table0+0x6f4>
   28f09:	ff 
   28f0a:	0f 84 b3 00 00 00    	je     28fc3 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1733>
   28f10:	48 8d 04 c5 00 00 00 	lea    0x0(,%rax,8),%rax
   28f17:	00 
   28f18:	4c 01 e8             	add    %r13,%rax
   28f1b:	48 8b 1d be a7 04 00 	mov    0x4a7be(%rip),%rbx        # 736e0 <_DYNAMIC+0x250>
   28f22:	48 8b 8c 24 38 01 00 	mov    0x138(%rsp),%rcx
   28f29:	00 
   28f2a:	48 8b b4 24 40 01 00 	mov    0x140(%rsp),%rsi
   28f31:	00 
   28f32:	31 d2                	xor    %edx,%edx
   28f34:	4d 89 e8             	mov    %r13,%r8
   28f37:	66 0f 1f 84 00 00 00 	nopw   0x0(%rax,%rax,1)
   28f3e:	00 00 
   28f40:	4c 8b 4c 24 48       	mov    0x48(%rsp),%r9
   28f45:	49 8d 3c 14          	lea    (%r12,%rdx,1),%rdi
   28f49:	4c 01 cf             	add    %r9,%rdi
   28f4c:	48 39 f7             	cmp    %rsi,%rdi
   28f4f:	0f 83 d1 07 00 00    	jae    29726 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1e96>
   28f55:	49 01 d1             	add    %rdx,%r9
   28f58:	45 31 d2             	xor    %r10d,%r10d
   28f5b:	49 83 f9 0c          	cmp    $0xc,%r9
   28f5f:	41 0f 93 c2          	setae  %r10b
   28f63:	f2 43 0f 10 04 d3    	movsd  (%r11,%r10,8),%xmm0
   28f69:	66 0f 28 c8          	movapd %xmm0,%xmm1
   28f6d:	66 0f 57 ca          	xorpd  %xmm2,%xmm1
   28f71:	f2 41 0f 5f 08       	maxsd  (%r8),%xmm1
   28f76:	49 83 c0 08          	add    $0x8,%r8
   28f7a:	f2 0f 5d c1          	minsd  %xmm1,%xmm0
   28f7e:	f2 0f 11 04 f9       	movsd  %xmm0,(%rcx,%rdi,8)
   28f83:	48 83 c2 03          	add    $0x3,%rdx
   28f87:	49 39 c0             	cmp    %rax,%r8
   28f8a:	75 b4                	jne    28f40 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x16b0>
   28f8c:	4d 85 ff             	test   %r15,%r15
   28f8f:	74 11                	je     28fa2 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1712>
   28f91:	49 c1 e7 03          	shl    $0x3,%r15
   28f95:	ba 08 00 00 00       	mov    $0x8,%edx
   28f9a:	4c 89 ef             	mov    %r13,%rdi
   28f9d:	4c 89 fe             	mov    %r15,%rsi
   28fa0:	ff d3                	call   *%rbx
   28fa2:	48 8b 74 24 28       	mov    0x28(%rsp),%rsi
   28fa7:	48 85 f6             	test   %rsi,%rsi
   28faa:	0f 84 f0 f7 ff ff    	je     287a0 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0xf10>
   28fb0:	48 c1 e6 03          	shl    $0x3,%rsi
   28fb4:	ba 08 00 00 00       	mov    $0x8,%edx
   28fb9:	48 89 ef             	mov    %rbp,%rdi
   28fbc:	ff d3                	call   *%rbx
   28fbe:	e9 dd f7 ff ff       	jmp    287a0 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0xf10>
   28fc3:	48 8b 1d 16 a7 04 00 	mov    0x4a716(%rip),%rbx        # 736e0 <_DYNAMIC+0x250>
   28fca:	4d 85 ff             	test   %r15,%r15
   28fcd:	75 c2                	jne    28f91 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1701>
   28fcf:	eb d1                	jmp    28fa2 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1712>
   28fd1:	48 8b 84 24 78 01 00 	mov    0x178(%rsp),%rax
   28fd8:	00 
   28fd9:	48 05 e0 e3 ff ff    	add    $0xffffffffffffe3e0,%rax
   28fdf:	48 8b b4 24 20 01 00 	mov    0x120(%rsp),%rsi
   28fe6:	00 
   28fe7:	48 81 c6 00 e1 00 00 	add    $0xe100,%rsi
   28fee:	48 8b 94 24 d0 01 00 	mov    0x1d0(%rsp),%rdx
   28ff5:	00 
   28ff6:	48 81 c2 20 1c 00 00 	add    $0x1c20,%rdx
   28ffd:	48 8b 8c 24 e0 01 00 	mov    0x1e0(%rsp),%rcx
   29004:	00 
   29005:	48 81 c1 70 08 00 00 	add    $0x870,%rcx
   2900c:	48 8b bc 24 18 01 00 	mov    0x118(%rsp),%rdi
   29013:	00 
   29014:	48 83 ff 0c          	cmp    $0xc,%rdi
   29018:	0f 85 a7 f6 ff ff    	jne    286c5 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0xe35>
   2901e:	ff 15 f4 a6 04 00    	call   *0x4a6f4(%rip)        # 73718 <_DYNAMIC+0x288>
   29024:	41 bf 80 ca 00 00    	mov    $0xca80,%r15d
   2902a:	41 bd 01 00 00 00    	mov    $0x1,%r13d
   29030:	bf 80 ca 00 00       	mov    $0xca80,%edi
   29035:	be 01 00 00 00       	mov    $0x1,%esi
   2903a:	ff 15 e0 a6 04 00    	call   *0x4a6e0(%rip)        # 73720 <_DYNAMIC+0x290>
   29040:	48 85 c0             	test   %rax,%rax
   29043:	0f 84 69 08 00 00    	je     298b2 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x2022>
   29049:	48 89 c5             	mov    %rax,%rbp
   2904c:	48 c7 44 24 28 80 ca 	movq   $0xca80,0x28(%rsp)
   29053:	00 00 
   29055:	48 89 44 24 30       	mov    %rax,0x30(%rsp)
   2905a:	48 c7 44 24 38 00 00 	movq   $0x0,0x38(%rsp)
   29061:	00 00 
   29063:	48 8b 84 24 38 01 00 	mov    0x138(%rsp),%rax
   2906a:	00 
   2906b:	48 8b 8c 24 40 01 00 	mov    0x140(%rsp),%rcx
   29072:	00 
   29073:	48 89 4c 24 20       	mov    %rcx,0x20(%rsp)
   29078:	48 89 84 24 50 01 00 	mov    %rax,0x150(%rsp)
   2907f:	00 
   29080:	4c 8d 60 10          	lea    0x10(%rax),%r12
   29084:	31 db                	xor    %ebx,%ebx
   29086:	45 31 ed             	xor    %r13d,%r13d
   29089:	31 d2                	xor    %edx,%edx
   2908b:	e9 9e 00 00 00       	jmp    2912e <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x189e>
   29090:	c6 07 02             	movb   $0x2,(%rdi)
   29093:	48 89 f8             	mov    %rdi,%rax
   29096:	48 81 c4 78 02 00 00 	add    $0x278,%rsp
   2909d:	5b                   	pop    %rbx
   2909e:	41 5c                	pop    %r12
   290a0:	41 5d                	pop    %r13
   290a2:	41 5e                	pop    %r14
   290a4:	41 5f                	pop    %r15
   290a6:	5d                   	pop    %rbp
   290a7:	c3                   	ret
   290a8:	49 8d 76 64          	lea    0x64(%r14),%rsi
   290ac:	48 8d 0d 05 81 04 00 	lea    0x48105(%rip),%rcx        # 711b8 <anon.e11926f7acc299ef6543ba90fd675eca.18.llvm.1662880743899104407+0x258>
   290b3:	ba 80 51 01 00       	mov    $0x15180,%edx
   290b8:	4c 89 f7             	mov    %r14,%rdi
   290bb:	eb 07                	jmp    290c4 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1834>
   290bd:	48 8d 0d dc 80 04 00 	lea    0x480dc(%rip),%rcx        # 711a0 <anon.e11926f7acc299ef6543ba90fd675eca.18.llvm.1662880743899104407+0x240>
   290c4:	ff 15 5e a6 04 00    	call   *0x4a65e(%rip)        # 73728 <_DYNAMIC+0x298>
   290ca:	e9 11 08 00 00       	jmp    298e0 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x2050>
   290cf:	48 01 ef             	add    %rbp,%rdi
   290d2:	48 8d 15 4f 80 04 00 	lea    0x4804f(%rip),%rdx        # 71128 <anon.e11926f7acc299ef6543ba90fd675eca.18.llvm.1662880743899104407+0x1c8>
   290d9:	be 80 51 01 00       	mov    $0x15180,%esi
   290de:	eb 12                	jmp    290f2 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1862>
   290e0:	49 01 ef             	add    %rbp,%r15
   290e3:	48 8d 15 56 80 04 00 	lea    0x48056(%rip),%rdx        # 71140 <anon.e11926f7acc299ef6543ba90fd675eca.18.llvm.1662880743899104407+0x1e0>
   290ea:	be 64 00 00 00       	mov    $0x64,%esi
   290ef:	4c 89 ff             	mov    %r15,%rdi
   290f2:	4c 8b 34 24          	mov    (%rsp),%r14
   290f6:	ff 15 bc a6 04 00    	call   *0x4a6bc(%rip)        # 737b8 <_DYNAMIC+0x328>
   290fc:	e9 df 07 00 00       	jmp    298e0 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x2050>
   29101:	4c 8b 64 24 70       	mov    0x70(%rsp),%r12
   29106:	49 81 c4 80 43 00 00 	add    $0x4380,%r12
   2910d:	48 8b 9c 24 90 01 00 	mov    0x190(%rsp),%rbx
   29114:	00 
   29115:	48 81 c3 70 08 00 00 	add    $0x870,%rbx
   2911c:	48 8b 94 24 80 01 00 	mov    0x180(%rsp),%rdx
   29123:	00 
   29124:	48 83 fa 0c          	cmp    $0xc,%rdx
   29128:	0f 84 21 06 00 00    	je     2974f <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1ebf>
   2912e:	48 ff c2             	inc    %rdx
   29131:	48 89 94 24 80 01 00 	mov    %rdx,0x180(%rsp)
   29138:	00 
   29139:	48 89 9c 24 90 01 00 	mov    %rbx,0x190(%rsp)
   29140:	00 
   29141:	4c 89 64 24 70       	mov    %r12,0x70(%rsp)
   29146:	31 c0                	xor    %eax,%eax
   29148:	eb 29                	jmp    29173 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x18e3>
   2914a:	4c 8b a4 24 90 00 00 	mov    0x90(%rsp),%r12
   29151:	00 
   29152:	49 81 c4 80 07 00 00 	add    $0x780,%r12
   29159:	48 8b 5c 24 50       	mov    0x50(%rsp),%rbx
   2915e:	48 81 c3 f0 00 00 00 	add    $0xf0,%rbx
   29165:	48 8b 84 24 00 01 00 	mov    0x100(%rsp),%rax
   2916c:	00 
   2916d:	48 83 f8 09          	cmp    $0x9,%rax
   29171:	74 8e                	je     29101 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1871>
   29173:	48 ff c0             	inc    %rax
   29176:	48 89 84 24 00 01 00 	mov    %rax,0x100(%rsp)
   2917d:	00 
   2917e:	48 89 5c 24 50       	mov    %rbx,0x50(%rsp)
   29183:	4c 89 a4 24 90 00 00 	mov    %r12,0x90(%rsp)
   2918a:	00 
   2918b:	31 c0                	xor    %eax,%eax
   2918d:	eb 17                	jmp    291a6 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1916>
   2918f:	90                   	nop
   29190:	49 81 c4 f0 00 00 00 	add    $0xf0,%r12
   29197:	48 83 c3 1e          	add    $0x1e,%rbx
   2919b:	48 8b 44 24 60       	mov    0x60(%rsp),%rax
   291a0:	48 83 f8 08          	cmp    $0x8,%rax
   291a4:	74 a4                	je     2914a <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x18ba>
   291a6:	48 ff c0             	inc    %rax
   291a9:	48 89 44 24 60       	mov    %rax,0x60(%rsp)
   291ae:	49 c7 c7 fd ff ff ff 	mov    $0xfffffffffffffffd,%r15
   291b5:	66 66 2e 0f 1f 84 00 	data16 cs nopw 0x0(%rax,%rax,1)
   291bc:	00 00 00 00 
   291c0:	66 0f 57 c0          	xorpd  %xmm0,%xmm0
   291c4:	f3 0f 11 04 24       	movss  %xmm0,(%rsp)
   291c9:	49 83 ff 08          	cmp    $0x8,%r15
   291cd:	77 12                	ja     291e1 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1951>
   291cf:	48 8d 05 be 6f fe ff 	lea    -0x19042(%rip),%rax        # 10194 <anon.e11926f7acc299ef6543ba90fd675eca.16.llvm.1662880743899104407+0x87e>
   291d6:	f3 42 0f 10 04 b8    	movss  (%rax,%r15,4),%xmm0
   291dc:	f3 0f 11 04 24       	movss  %xmm0,(%rsp)
   291e1:	4a 8d 3c 3b          	lea    (%rbx,%r15,1),%rdi
   291e5:	48 83 c7 03          	add    $0x3,%rdi
   291e9:	48 3b 7c 24 20       	cmp    0x20(%rsp),%rdi
   291ee:	0f 83 44 05 00 00    	jae    29738 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1ea8>
   291f4:	f2 43 0f 10 44 fc 08 	movsd  0x8(%r12,%r15,8),%xmm0
   291fb:	f2 0f 5a c0          	cvtsd2ss %xmm0,%xmm0
   291ff:	f3 0f 10 0c 24       	movss  (%rsp),%xmm1
   29204:	f3 0f 58 c8          	addss  %xmm0,%xmm1
   29208:	f3 0f 11 0c 24       	movss  %xmm1,(%rsp)
   2920d:	48 8b 05 5c a8 04 00 	mov    0x4a85c(%rip),%rax        # 73a70 <_DYNAMIC+0x5e0>
   29214:	48 8b 00             	mov    (%rax),%rax
   29217:	48 85 c0             	test   %rax,%rax
   2921a:	0f 84 ec 00 00 00    	je     2930c <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1a7c>
   29220:	48 b9 00 00 00 00 00 	movabs $0x40000000000000,%rcx
   29227:	00 40 00 
   2922a:	48 85 c8             	test   %rcx,%rax
   2922d:	0f 85 ed 00 00 00    	jne    29320 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1a90>
   29233:	66 0f 6e 04 24       	movd   (%rsp),%xmm0
   29238:	66 0f 7e c1          	movd   %xmm0,%ecx
   2923c:	89 c8                	mov    %ecx,%eax
   2923e:	25 00 00 00 80       	and    $0x80000000,%eax
   29243:	89 ce                	mov    %ecx,%esi
   29245:	81 e6 00 00 80 7f    	and    $0x7f800000,%esi
   2924b:	89 ca                	mov    %ecx,%edx
   2924d:	81 e2 ff ff 7f 00    	and    $0x7fffff,%edx
   29253:	81 fe 00 00 80 7f    	cmp    $0x7f800000,%esi
   29259:	75 25                	jne    29280 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x19f0>
   2925b:	31 c9                	xor    %ecx,%ecx
   2925d:	85 d2                	test   %edx,%edx
   2925f:	0f 95 c1             	setne  %cl
   29262:	c1 e1 09             	shl    $0x9,%ecx
   29265:	c1 e8 10             	shr    $0x10,%eax
   29268:	c1 ea 0d             	shr    $0xd,%edx
   2926b:	09 c2                	or     %eax,%edx
   2926d:	09 ca                	or     %ecx,%edx
   2926f:	81 ca 00 7c 00 00    	or     $0x7c00,%edx
   29275:	41 89 d6             	mov    %edx,%r14d
   29278:	e9 b0 00 00 00       	jmp    2932d <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1a9d>
   2927d:	0f 1f 00             	nopl   (%rax)
   29280:	c1 e8 10             	shr    $0x10,%eax
   29283:	81 fe 00 00 00 47    	cmp    $0x47000000,%esi
   29289:	76 0d                	jbe    29298 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1a08>
   2928b:	0d 00 7c 00 00       	or     $0x7c00,%eax
   29290:	41 89 c6             	mov    %eax,%r14d
   29293:	e9 95 00 00 00       	jmp    2932d <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1a9d>
   29298:	81 fe 00 00 80 38    	cmp    $0x38800000,%esi
   2929e:	73 42                	jae    292e2 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1a52>
   292a0:	81 fe 00 00 00 33    	cmp    $0x33000000,%esi
   292a6:	72 e8                	jb     29290 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1a00>
   292a8:	c1 ee 17             	shr    $0x17,%esi
   292ab:	81 ca 00 00 80 00    	or     $0x800000,%edx
   292b1:	b1 7e                	mov    $0x7e,%cl
   292b3:	40 28 f1             	sub    %sil,%cl
   292b6:	41 89 d6             	mov    %edx,%r14d
   292b9:	41 d3 ee             	shr    %cl,%r14d
   292bc:	b9 1d 00 00 00       	mov    $0x1d,%ecx
   292c1:	29 f1                	sub    %esi,%ecx
   292c3:	83 e1 1f             	and    $0x1f,%ecx
   292c6:	0f a3 ca             	bt     %ecx,%edx
   292c9:	73 12                	jae    292dd <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1a4d>
   292cb:	be 03 00 00 00       	mov    $0x3,%esi
   292d0:	d3 e6                	shl    %cl,%esi
   292d2:	ff ce                	dec    %esi
   292d4:	21 d6                	and    %edx,%esi
   292d6:	83 fe 01             	cmp    $0x1,%esi
   292d9:	41 83 de ff          	sbb    $0xffffffff,%r14d
   292dd:	41 09 c6             	or     %eax,%r14d
   292e0:	eb 4b                	jmp    2932d <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1a9d>
   292e2:	c1 ee 0d             	shr    $0xd,%esi
   292e5:	c1 ea 0d             	shr    $0xd,%edx
   292e8:	89 cf                	mov    %ecx,%edi
   292ea:	c1 ef 0c             	shr    $0xc,%edi
   292ed:	f7 c1 ff 2f 00 00    	test   $0x2fff,%ecx
   292f3:	0f 95 c1             	setne  %cl
   292f6:	40 20 f9             	and    %dil,%cl
   292f9:	01 f2                	add    %esi,%edx
   292fb:	81 c2 00 40 00 00    	add    $0x4000,%edx
   29301:	09 c2                	or     %eax,%edx
   29303:	44 0f b6 f1          	movzbl %cl,%r14d
   29307:	41 01 d6             	add    %edx,%r14d
   2930a:	eb 21                	jmp    2932d <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1a9d>
   2930c:	ff 15 66 a7 04 00    	call   *0x4a766(%rip)        # 73a78 <_DYNAMIC+0x5e8>
   29312:	48 0f ba e0 36       	bt     $0x36,%rax
   29317:	0f 83 16 ff ff ff    	jae    29233 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x19a3>
   2931d:	0f 1f 00             	nopl   (%rax)
   29320:	66 0f 6e 04 24       	movd   (%rsp),%xmm0
   29325:	e8 d6 06 00 00       	call   29a00 <half::binary16::arch::x86::f32_to_f16_x86_f16c>
   2932a:	41 89 c6             	mov    %eax,%r14d
   2932d:	48 8b 44 24 28       	mov    0x28(%rsp),%rax
   29332:	4c 29 e8             	sub    %r13,%rax
   29335:	48 83 f8 01          	cmp    $0x1,%rax
   29339:	0f 86 68 01 00 00    	jbe    294a7 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1c17>
   2933f:	66 46 89 74 2d 00    	mov    %r14w,0x0(%rbp,%r13,1)
   29345:	49 83 c5 02          	add    $0x2,%r13
   29349:	4c 89 6c 24 38       	mov    %r13,0x38(%rsp)
   2934e:	49 8d 47 01          	lea    0x1(%r15),%rax
   29352:	66 0f ef c0          	pxor   %xmm0,%xmm0
   29356:	66 0f 7e 04 24       	movd   %xmm0,(%rsp)
   2935b:	48 83 f8 08          	cmp    $0x8,%rax
   2935f:	77 13                	ja     29374 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1ae4>
   29361:	48 8d 05 2c 6e fe ff 	lea    -0x191d4(%rip),%rax        # 10194 <anon.e11926f7acc299ef6543ba90fd675eca.16.llvm.1662880743899104407+0x87e>
   29368:	66 42 0f 6e 44 b8 04 	movd   0x4(%rax,%r15,4),%xmm0
   2936f:	66 0f 7e 04 24       	movd   %xmm0,(%rsp)
   29374:	4a 8d 3c 3b          	lea    (%rbx,%r15,1),%rdi
   29378:	48 83 c7 04          	add    $0x4,%rdi
   2937c:	48 3b 7c 24 20       	cmp    0x20(%rsp),%rdi
   29381:	0f 83 b1 03 00 00    	jae    29738 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1ea8>
   29387:	f2 43 0f 10 44 fc 10 	movsd  0x10(%r12,%r15,8),%xmm0
   2938e:	f2 0f 5a c0          	cvtsd2ss %xmm0,%xmm0
   29392:	f3 0f 10 0c 24       	movss  (%rsp),%xmm1
   29397:	f3 0f 58 c8          	addss  %xmm0,%xmm1
   2939b:	f3 0f 11 0c 24       	movss  %xmm1,(%rsp)
   293a0:	48 8b 05 c9 a6 04 00 	mov    0x4a6c9(%rip),%rax        # 73a70 <_DYNAMIC+0x5e0>
   293a7:	48 8b 00             	mov    (%rax),%rax
   293aa:	48 85 c0             	test   %rax,%rax
   293ad:	0f 84 20 01 00 00    	je     294d3 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1c43>
   293b3:	48 b9 00 00 00 00 00 	movabs $0x40000000000000,%rcx
   293ba:	00 40 00 
   293bd:	48 85 c8             	test   %rcx,%rax
   293c0:	0f 85 2a 01 00 00    	jne    294f0 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1c60>
   293c6:	66 0f 6e 04 24       	movd   (%rsp),%xmm0
   293cb:	66 0f 7e c1          	movd   %xmm0,%ecx
   293cf:	89 c8                	mov    %ecx,%eax
   293d1:	25 00 00 00 80       	and    $0x80000000,%eax
   293d6:	89 ce                	mov    %ecx,%esi
   293d8:	81 e6 00 00 80 7f    	and    $0x7f800000,%esi
   293de:	89 ca                	mov    %ecx,%edx
   293e0:	81 e2 ff ff 7f 00    	and    $0x7fffff,%edx
   293e6:	81 fe 00 00 80 7f    	cmp    $0x7f800000,%esi
   293ec:	75 22                	jne    29410 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1b80>
   293ee:	31 c9                	xor    %ecx,%ecx
   293f0:	85 d2                	test   %edx,%edx
   293f2:	0f 95 c1             	setne  %cl
   293f5:	c1 e1 09             	shl    $0x9,%ecx
   293f8:	c1 e8 10             	shr    $0x10,%eax
   293fb:	c1 ea 0d             	shr    $0xd,%edx
   293fe:	09 c2                	or     %eax,%edx
   29400:	09 ca                	or     %ecx,%edx
   29402:	81 ca 00 7c 00 00    	or     $0x7c00,%edx
   29408:	41 89 d6             	mov    %edx,%r14d
   2940b:	e9 ed 00 00 00       	jmp    294fd <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1c6d>
   29410:	c1 e8 10             	shr    $0x10,%eax
   29413:	81 fe 00 00 00 47    	cmp    $0x47000000,%esi
   29419:	76 0d                	jbe    29428 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1b98>
   2941b:	0d 00 7c 00 00       	or     $0x7c00,%eax
   29420:	41 89 c6             	mov    %eax,%r14d
   29423:	e9 d5 00 00 00       	jmp    294fd <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1c6d>
   29428:	81 fe 00 00 80 38    	cmp    $0x38800000,%esi
   2942e:	73 10                	jae    29440 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1bb0>
   29430:	81 fe 00 00 00 33    	cmp    $0x33000000,%esi
   29436:	73 35                	jae    2946d <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1bdd>
   29438:	41 89 c6             	mov    %eax,%r14d
   2943b:	e9 bd 00 00 00       	jmp    294fd <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1c6d>
   29440:	c1 ee 0d             	shr    $0xd,%esi
   29443:	c1 ea 0d             	shr    $0xd,%edx
   29446:	89 cf                	mov    %ecx,%edi
   29448:	c1 ef 0c             	shr    $0xc,%edi
   2944b:	f7 c1 ff 2f 00 00    	test   $0x2fff,%ecx
   29451:	0f 95 c1             	setne  %cl
   29454:	40 20 f9             	and    %dil,%cl
   29457:	01 f2                	add    %esi,%edx
   29459:	81 c2 00 40 00 00    	add    $0x4000,%edx
   2945f:	09 c2                	or     %eax,%edx
   29461:	44 0f b6 f1          	movzbl %cl,%r14d
   29465:	41 01 d6             	add    %edx,%r14d
   29468:	e9 90 00 00 00       	jmp    294fd <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1c6d>
   2946d:	c1 ee 17             	shr    $0x17,%esi
   29470:	81 ca 00 00 80 00    	or     $0x800000,%edx
   29476:	b1 7e                	mov    $0x7e,%cl
   29478:	40 28 f1             	sub    %sil,%cl
   2947b:	41 89 d6             	mov    %edx,%r14d
   2947e:	41 d3 ee             	shr    %cl,%r14d
   29481:	b9 1d 00 00 00       	mov    $0x1d,%ecx
   29486:	29 f1                	sub    %esi,%ecx
   29488:	83 e1 1f             	and    $0x1f,%ecx
   2948b:	0f a3 ca             	bt     %ecx,%edx
   2948e:	73 12                	jae    294a2 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1c12>
   29490:	be 03 00 00 00       	mov    $0x3,%esi
   29495:	d3 e6                	shl    %cl,%esi
   29497:	ff ce                	dec    %esi
   29499:	21 d6                	and    %edx,%esi
   2949b:	83 fe 01             	cmp    $0x1,%esi
   2949e:	41 83 de ff          	sbb    $0xffffffff,%r14d
   294a2:	41 09 c6             	or     %eax,%r14d
   294a5:	eb 56                	jmp    294fd <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1c6d>
   294a7:	ba 02 00 00 00       	mov    $0x2,%edx
   294ac:	b9 01 00 00 00       	mov    $0x1,%ecx
   294b1:	41 b8 01 00 00 00    	mov    $0x1,%r8d
   294b7:	48 8d 7c 24 28       	lea    0x28(%rsp),%rdi
   294bc:	4c 89 ee             	mov    %r13,%rsi
   294bf:	e8 2c 08 00 00       	call   29cf0 <<alloc::raw_vec::RawVecInner<_>>::reserve::do_reserve_and_handle::<alloc::alloc::Global>>
   294c4:	48 8b 6c 24 30       	mov    0x30(%rsp),%rbp
   294c9:	4c 8b 6c 24 38       	mov    0x38(%rsp),%r13
   294ce:	e9 6c fe ff ff       	jmp    2933f <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1aaf>
   294d3:	ff 15 9f a5 04 00    	call   *0x4a59f(%rip)        # 73a78 <_DYNAMIC+0x5e8>
   294d9:	48 0f ba e0 36       	bt     $0x36,%rax
   294de:	0f 83 e2 fe ff ff    	jae    293c6 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1b36>
   294e4:	66 66 66 2e 0f 1f 84 	data16 data16 cs nopw 0x0(%rax,%rax,1)
   294eb:	00 00 00 00 00 
   294f0:	66 0f 6e 04 24       	movd   (%rsp),%xmm0
   294f5:	e8 06 05 00 00       	call   29a00 <half::binary16::arch::x86::f32_to_f16_x86_f16c>
   294fa:	41 89 c6             	mov    %eax,%r14d
   294fd:	48 8b 44 24 28       	mov    0x28(%rsp),%rax
   29502:	4c 29 e8             	sub    %r13,%rax
   29505:	48 83 f8 01          	cmp    $0x1,%rax
   29509:	0f 86 78 01 00 00    	jbe    29687 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1df7>
   2950f:	48 8b 6c 24 30       	mov    0x30(%rsp),%rbp
   29514:	66 46 89 74 2d 00    	mov    %r14w,0x0(%rbp,%r13,1)
   2951a:	49 83 c5 02          	add    $0x2,%r13
   2951e:	4c 89 6c 24 38       	mov    %r13,0x38(%rsp)
   29523:	49 8d 47 02          	lea    0x2(%r15),%rax
   29527:	66 0f ef c0          	pxor   %xmm0,%xmm0
   2952b:	66 0f 7e 04 24       	movd   %xmm0,(%rsp)
   29530:	48 83 f8 08          	cmp    $0x8,%rax
   29534:	77 13                	ja     29549 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1cb9>
   29536:	48 8d 05 57 6c fe ff 	lea    -0x193a9(%rip),%rax        # 10194 <anon.e11926f7acc299ef6543ba90fd675eca.16.llvm.1662880743899104407+0x87e>
   2953d:	66 42 0f 6e 44 b8 08 	movd   0x8(%rax,%r15,4),%xmm0
   29544:	66 0f 7e 04 24       	movd   %xmm0,(%rsp)
   29549:	4a 8d 3c 3b          	lea    (%rbx,%r15,1),%rdi
   2954d:	48 83 c7 05          	add    $0x5,%rdi
   29551:	48 3b 7c 24 20       	cmp    0x20(%rsp),%rdi
   29556:	0f 83 dc 01 00 00    	jae    29738 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1ea8>
   2955c:	f2 43 0f 10 44 fc 18 	movsd  0x18(%r12,%r15,8),%xmm0
   29563:	f2 0f 5a c0          	cvtsd2ss %xmm0,%xmm0
   29567:	f3 0f 10 0c 24       	movss  (%rsp),%xmm1
   2956c:	f3 0f 58 c8          	addss  %xmm0,%xmm1
   29570:	f3 0f 11 0c 24       	movss  %xmm1,(%rsp)
   29575:	48 8b 05 f4 a4 04 00 	mov    0x4a4f4(%rip),%rax        # 73a70 <_DYNAMIC+0x5e0>
   2957c:	48 8b 00             	mov    (%rax),%rax
   2957f:	48 85 c0             	test   %rax,%rax
   29582:	0f 84 26 01 00 00    	je     296ae <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1e1e>
   29588:	48 b9 00 00 00 00 00 	movabs $0x40000000000000,%rcx
   2958f:	00 40 00 
   29592:	48 85 c8             	test   %rcx,%rax
   29595:	0f 85 25 01 00 00    	jne    296c0 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1e30>
   2959b:	66 0f 6e 04 24       	movd   (%rsp),%xmm0
   295a0:	66 0f 7e c1          	movd   %xmm0,%ecx
   295a4:	89 c8                	mov    %ecx,%eax
   295a6:	25 00 00 00 80       	and    $0x80000000,%eax
   295ab:	89 ce                	mov    %ecx,%esi
   295ad:	81 e6 00 00 80 7f    	and    $0x7f800000,%esi
   295b3:	89 ca                	mov    %ecx,%edx
   295b5:	81 e2 ff ff 7f 00    	and    $0x7fffff,%edx
   295bb:	81 fe 00 00 80 7f    	cmp    $0x7f800000,%esi
   295c1:	75 2d                	jne    295f0 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1d60>
   295c3:	31 c9                	xor    %ecx,%ecx
   295c5:	85 d2                	test   %edx,%edx
   295c7:	0f 95 c1             	setne  %cl
   295ca:	c1 e1 09             	shl    $0x9,%ecx
   295cd:	c1 e8 10             	shr    $0x10,%eax
   295d0:	c1 ea 0d             	shr    $0xd,%edx
   295d3:	09 c2                	or     %eax,%edx
   295d5:	09 ca                	or     %ecx,%edx
   295d7:	81 ca 00 7c 00 00    	or     $0x7c00,%edx
   295dd:	41 89 d6             	mov    %edx,%r14d
   295e0:	e9 e8 00 00 00       	jmp    296cd <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1e3d>
   295e5:	66 66 2e 0f 1f 84 00 	data16 cs nopw 0x0(%rax,%rax,1)
   295ec:	00 00 00 00 
   295f0:	c1 e8 10             	shr    $0x10,%eax
   295f3:	81 fe 00 00 00 47    	cmp    $0x47000000,%esi
   295f9:	76 0d                	jbe    29608 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1d78>
   295fb:	0d 00 7c 00 00       	or     $0x7c00,%eax
   29600:	41 89 c6             	mov    %eax,%r14d
   29603:	e9 c5 00 00 00       	jmp    296cd <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1e3d>
   29608:	81 fe 00 00 80 38    	cmp    $0x38800000,%esi
   2960e:	73 10                	jae    29620 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1d90>
   29610:	81 fe 00 00 00 33    	cmp    $0x33000000,%esi
   29616:	73 35                	jae    2964d <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1dbd>
   29618:	41 89 c6             	mov    %eax,%r14d
   2961b:	e9 ad 00 00 00       	jmp    296cd <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1e3d>
   29620:	c1 ee 0d             	shr    $0xd,%esi
   29623:	c1 ea 0d             	shr    $0xd,%edx
   29626:	89 cf                	mov    %ecx,%edi
   29628:	c1 ef 0c             	shr    $0xc,%edi
   2962b:	f7 c1 ff 2f 00 00    	test   $0x2fff,%ecx
   29631:	0f 95 c1             	setne  %cl
   29634:	40 20 f9             	and    %dil,%cl
   29637:	01 f2                	add    %esi,%edx
   29639:	81 c2 00 40 00 00    	add    $0x4000,%edx
   2963f:	09 c2                	or     %eax,%edx
   29641:	44 0f b6 f1          	movzbl %cl,%r14d
   29645:	41 01 d6             	add    %edx,%r14d
   29648:	e9 80 00 00 00       	jmp    296cd <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1e3d>
   2964d:	c1 ee 17             	shr    $0x17,%esi
   29650:	81 ca 00 00 80 00    	or     $0x800000,%edx
   29656:	b1 7e                	mov    $0x7e,%cl
   29658:	40 28 f1             	sub    %sil,%cl
   2965b:	41 89 d6             	mov    %edx,%r14d
   2965e:	41 d3 ee             	shr    %cl,%r14d
   29661:	b9 1d 00 00 00       	mov    $0x1d,%ecx
   29666:	29 f1                	sub    %esi,%ecx
   29668:	83 e1 1f             	and    $0x1f,%ecx
   2966b:	0f a3 ca             	bt     %ecx,%edx
   2966e:	73 12                	jae    29682 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1df2>
   29670:	be 03 00 00 00       	mov    $0x3,%esi
   29675:	d3 e6                	shl    %cl,%esi
   29677:	ff ce                	dec    %esi
   29679:	21 d6                	and    %edx,%esi
   2967b:	83 fe 01             	cmp    $0x1,%esi
   2967e:	41 83 de ff          	sbb    $0xffffffff,%r14d
   29682:	41 09 c6             	or     %eax,%r14d
   29685:	eb 46                	jmp    296cd <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1e3d>
   29687:	ba 02 00 00 00       	mov    $0x2,%edx
   2968c:	b9 01 00 00 00       	mov    $0x1,%ecx
   29691:	41 b8 01 00 00 00    	mov    $0x1,%r8d
   29697:	48 8d 7c 24 28       	lea    0x28(%rsp),%rdi
   2969c:	4c 89 ee             	mov    %r13,%rsi
   2969f:	e8 4c 06 00 00       	call   29cf0 <<alloc::raw_vec::RawVecInner<_>>::reserve::do_reserve_and_handle::<alloc::alloc::Global>>
   296a4:	4c 8b 6c 24 38       	mov    0x38(%rsp),%r13
   296a9:	e9 61 fe ff ff       	jmp    2950f <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1c7f>
   296ae:	ff 15 c4 a3 04 00    	call   *0x4a3c4(%rip)        # 73a78 <_DYNAMIC+0x5e8>
   296b4:	48 0f ba e0 36       	bt     $0x36,%rax
   296b9:	0f 83 dc fe ff ff    	jae    2959b <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1d0b>
   296bf:	90                   	nop
   296c0:	66 0f 6e 04 24       	movd   (%rsp),%xmm0
   296c5:	e8 36 03 00 00       	call   29a00 <half::binary16::arch::x86::f32_to_f16_x86_f16c>
   296ca:	41 89 c6             	mov    %eax,%r14d
   296cd:	48 8b 44 24 28       	mov    0x28(%rsp),%rax
   296d2:	4c 29 e8             	sub    %r13,%rax
   296d5:	48 83 f8 01          	cmp    $0x1,%rax
   296d9:	76 22                	jbe    296fd <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1e6d>
   296db:	66 46 89 74 2d 00    	mov    %r14w,0x0(%rbp,%r13,1)
   296e1:	49 83 c5 02          	add    $0x2,%r13
   296e5:	4c 89 6c 24 38       	mov    %r13,0x38(%rsp)
   296ea:	49 83 c7 03          	add    $0x3,%r15
   296ee:	49 83 ff 1b          	cmp    $0x1b,%r15
   296f2:	0f 85 c8 fa ff ff    	jne    291c0 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1930>
   296f8:	e9 93 fa ff ff       	jmp    29190 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1900>
   296fd:	ba 02 00 00 00       	mov    $0x2,%edx
   29702:	b9 01 00 00 00       	mov    $0x1,%ecx
   29707:	41 b8 01 00 00 00    	mov    $0x1,%r8d
   2970d:	48 8d 7c 24 28       	lea    0x28(%rsp),%rdi
   29712:	4c 89 ee             	mov    %r13,%rsi
   29715:	e8 d6 05 00 00       	call   29cf0 <<alloc::raw_vec::RawVecInner<_>>::reserve::do_reserve_and_handle::<alloc::alloc::Global>>
   2971a:	48 8b 6c 24 30       	mov    0x30(%rsp),%rbp
   2971f:	4c 8b 6c 24 38       	mov    0x38(%rsp),%r13
   29724:	eb b5                	jmp    296db <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1e4b>
   29726:	48 8d 15 e3 79 04 00 	lea    0x479e3(%rip),%rdx        # 71110 <anon.e11926f7acc299ef6543ba90fd675eca.18.llvm.1662880743899104407+0x1b0>
   2972d:	ff 15 85 a0 04 00    	call   *0x4a085(%rip)        # 737b8 <_DYNAMIC+0x328>
   29733:	e9 a8 01 00 00       	jmp    298e0 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x2050>
   29738:	48 8d 15 b9 79 04 00 	lea    0x479b9(%rip),%rdx        # 710f8 <anon.e11926f7acc299ef6543ba90fd675eca.18.llvm.1662880743899104407+0x198>
   2973f:	48 8b 74 24 20       	mov    0x20(%rsp),%rsi
   29744:	ff 15 6e a0 04 00    	call   *0x4a06e(%rip)        # 737b8 <_DYNAMIC+0x328>
   2974a:	e9 91 01 00 00       	jmp    298e0 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x2050>
   2974f:	48 8d bc 24 b0 00 00 	lea    0xb0(%rsp),%rdi
   29756:	00 
   29757:	48 89 ee             	mov    %rbp,%rsi
   2975a:	4c 89 ea             	mov    %r13,%rdx
   2975d:	ff 15 35 a3 04 00    	call   *0x4a335(%rip)        # 73a98 <_DYNAMIC+0x608>
   29763:	83 bc 24 b0 00 00 00 	cmpl   $0x1,0xb0(%rsp)
   2976a:	01 
   2976b:	75 79                	jne    297e6 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1f56>
   2976d:	48 8b 84 24 b8 00 00 	mov    0xb8(%rsp),%rax
   29774:	00 
   29775:	f2 0f 10 84 24 c0 00 	movsd  0xc0(%rsp),%xmm0
   2977c:	00 00 
   2977e:	f3 0f 10 8c 24 d0 00 	movss  0xd0(%rsp),%xmm1
   29785:	00 00 
   29787:	0f b6 8c 24 d4 00 00 	movzbl 0xd4(%rsp),%ecx
   2978e:	00 
   2978f:	0f b6 94 24 d7 00 00 	movzbl 0xd7(%rsp),%edx
   29796:	00 
   29797:	48 8b b4 24 10 01 00 	mov    0x110(%rsp),%rsi
   2979e:	00 
   2979f:	88 56 1f             	mov    %dl,0x1f(%rsi)
   297a2:	0f b7 94 24 d5 00 00 	movzwl 0xd5(%rsp),%edx
   297a9:	00 
   297aa:	66 89 56 1d          	mov    %dx,0x1d(%rsi)
   297ae:	f2 0f 10 94 24 c8 00 	movsd  0xc8(%rsp),%xmm2
   297b5:	00 00 
   297b7:	48 89 06             	mov    %rax,(%rsi)
   297ba:	f2 0f 11 46 08       	movsd  %xmm0,0x8(%rsi)
   297bf:	f2 0f 11 56 10       	movsd  %xmm2,0x10(%rsi)
   297c4:	f3 0f 11 4e 18       	movss  %xmm1,0x18(%rsi)
   297c9:	88 4e 1c             	mov    %cl,0x1c(%rsi)
   297cc:	48 8b 74 24 28       	mov    0x28(%rsp),%rsi
   297d1:	48 85 f6             	test   %rsi,%rsi
   297d4:	74 2d                	je     29803 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1f73>
   297d6:	ba 01 00 00 00       	mov    $0x1,%edx
   297db:	48 89 ef             	mov    %rbp,%rdi
   297de:	ff 15 fc 9e 04 00    	call   *0x49efc(%rip)        # 736e0 <_DYNAMIC+0x250>
   297e4:	eb 1d                	jmp    29803 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1f73>
   297e6:	48 8b 44 24 38       	mov    0x38(%rsp),%rax
   297eb:	48 8b 8c 24 10 01 00 	mov    0x110(%rsp),%rcx
   297f2:	00 
   297f3:	48 89 41 18          	mov    %rax,0x18(%rcx)
   297f7:	0f 10 44 24 28       	movups 0x28(%rsp),%xmm0
   297fc:	0f 11 41 08          	movups %xmm0,0x8(%rcx)
   29800:	c6 01 ff             	movb   $0xff,(%rcx)
   29803:	48 8b b4 24 30 01 00 	mov    0x130(%rsp),%rsi
   2980a:	00 
   2980b:	48 85 f6             	test   %rsi,%rsi
   2980e:	74 17                	je     29827 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1f97>
   29810:	48 c1 e6 03          	shl    $0x3,%rsi
   29814:	ba 08 00 00 00       	mov    $0x8,%edx
   29819:	48 8b bc 24 50 01 00 	mov    0x150(%rsp),%rdi
   29820:	00 
   29821:	ff 15 b9 9e 04 00    	call   *0x49eb9(%rip)        # 736e0 <_DYNAMIC+0x250>
   29827:	48 8b 74 24 78       	mov    0x78(%rsp),%rsi
   2982c:	48 85 f6             	test   %rsi,%rsi
   2982f:	74 17                	je     29848 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1fb8>
   29831:	48 8b bc 24 80 00 00 	mov    0x80(%rsp),%rdi
   29838:	00 
   29839:	48 c1 e6 03          	shl    $0x3,%rsi
   2983d:	ba 08 00 00 00       	mov    $0x8,%edx
   29842:	ff 15 98 9e 04 00    	call   *0x49e98(%rip)        # 736e0 <_DYNAMIC+0x250>
   29848:	be 00 8c 0a 00       	mov    $0xa8c00,%esi
   2984d:	ba 08 00 00 00       	mov    $0x8,%edx
   29852:	48 8b 7c 24 18       	mov    0x18(%rsp),%rdi
   29857:	ff 15 83 9e 04 00    	call   *0x49e83(%rip)        # 736e0 <_DYNAMIC+0x250>
   2985d:	48 8b bc 24 10 01 00 	mov    0x110(%rsp),%rdi
   29864:	00 
   29865:	e9 29 f8 ff ff       	jmp    29093 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1803>
   2986a:	48 89 c7             	mov    %rax,%rdi
   2986d:	48 8d 05 fc 78 04 00 	lea    0x478fc(%rip),%rax        # 71170 <anon.e11926f7acc299ef6543ba90fd675eca.18.llvm.1662880743899104407+0x210>
   29874:	eb 0a                	jmp    29880 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1ff0>
   29876:	48 89 c7             	mov    %rax,%rdi
   29879:	48 8d 05 08 79 04 00 	lea    0x47908(%rip),%rax        # 71188 <anon.e11926f7acc299ef6543ba90fd675eca.18.llvm.1662880743899104407+0x228>
   29880:	48 89 84 24 68 01 00 	mov    %rax,0x168(%rsp)
   29887:	00 
   29888:	48 8b b4 24 a8 01 00 	mov    0x1a8(%rsp),%rsi
   2988f:	00 
   29890:	48 8b 94 24 68 01 00 	mov    0x168(%rsp),%rdx
   29897:	00 
   29898:	ff 15 1a 9f 04 00    	call   *0x49f1a(%rip)        # 737b8 <_DYNAMIC+0x328>
   2989e:	eb 40                	jmp    298e0 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x2050>
   298a0:	bf 08 00 00 00       	mov    $0x8,%edi
   298a5:	be 50 00 00 00       	mov    $0x50,%esi
   298aa:	ff 15 60 9e 04 00    	call   *0x49e60(%rip)        # 73710 <_DYNAMIC+0x280>
   298b0:	eb 2e                	jmp    298e0 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x2050>
   298b2:	4c 89 ef             	mov    %r13,%rdi
   298b5:	4c 89 fe             	mov    %r15,%rsi
   298b8:	ff 15 52 9e 04 00    	call   *0x49e52(%rip)        # 73710 <_DYNAMIC+0x280>
   298be:	eb 20                	jmp    298e0 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x2050>
   298c0:	bf 08 00 00 00       	mov    $0x8,%edi
   298c5:	be 00 8c 0a 00       	mov    $0xa8c00,%esi
   298ca:	ff 15 40 9e 04 00    	call   *0x49e40(%rip)        # 73710 <_DYNAMIC+0x280>
   298d0:	bf 08 00 00 00       	mov    $0x8,%edi
   298d5:	be 00 2a 03 00       	mov    $0x32a00,%esi
   298da:	ff 15 30 9e 04 00    	call   *0x49e30(%rip)        # 73710 <_DYNAMIC+0x280>
   298e0:	0f 0b                	ud2
   298e2:	eb 2a                	jmp    2990e <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x207e>
   298e4:	49 89 c4             	mov    %rax,%r12
   298e7:	e9 d8 00 00 00       	jmp    299c4 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x2134>
   298ec:	e9 af 00 00 00       	jmp    299a0 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x2110>
   298f1:	49 89 c4             	mov    %rax,%r12
   298f4:	e9 81 00 00 00       	jmp    2997a <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x20ea>
   298f9:	e9 a2 00 00 00       	jmp    299a0 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x2110>
   298fe:	49 89 c4             	mov    %rax,%r12
   29901:	4c 8b 34 24          	mov    (%rsp),%r14
   29905:	eb 3d                	jmp    29944 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x20b4>
   29907:	eb 5b                	jmp    29964 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x20d4>
   29909:	49 89 c4             	mov    %rax,%r12
   2990c:	eb 36                	jmp    29944 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x20b4>
   2990e:	49 89 c4             	mov    %rax,%r12
   29911:	48 8b 74 24 28       	mov    0x28(%rsp),%rsi
   29916:	48 85 f6             	test   %rsi,%rsi
   29919:	74 5f                	je     2997a <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x20ea>
   2991b:	48 8b 7c 24 30       	mov    0x30(%rsp),%rdi
   29920:	ba 01 00 00 00       	mov    $0x1,%edx
   29925:	eb 4d                	jmp    29974 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x20e4>
   29927:	49 89 c4             	mov    %rax,%r12
   2992a:	4d 85 ff             	test   %r15,%r15
   2992d:	74 15                	je     29944 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x20b4>
   2992f:	49 c1 e7 03          	shl    $0x3,%r15
   29933:	ba 08 00 00 00       	mov    $0x8,%edx
   29938:	4c 89 ef             	mov    %r13,%rdi
   2993b:	4c 89 fe             	mov    %r15,%rsi
   2993e:	ff 15 9c 9d 04 00    	call   *0x49d9c(%rip)        # 736e0 <_DYNAMIC+0x250>
   29944:	48 8b 74 24 28       	mov    0x28(%rsp),%rsi
   29949:	48 85 f6             	test   %rsi,%rsi
   2994c:	74 19                	je     29967 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x20d7>
   2994e:	48 8b 7c 24 30       	mov    0x30(%rsp),%rdi
   29953:	48 c1 e6 03          	shl    $0x3,%rsi
   29957:	ba 08 00 00 00       	mov    $0x8,%edx
   2995c:	ff 15 7e 9d 04 00    	call   *0x49d7e(%rip)        # 736e0 <_DYNAMIC+0x250>
   29962:	eb 03                	jmp    29967 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x20d7>
   29964:	49 89 c4             	mov    %rax,%r12
   29967:	be 20 03 00 00       	mov    $0x320,%esi
   2996c:	ba 08 00 00 00       	mov    $0x8,%edx
   29971:	4c 89 f7             	mov    %r14,%rdi
   29974:	ff 15 66 9d 04 00    	call   *0x49d66(%rip)        # 736e0 <_DYNAMIC+0x250>
   2997a:	48 8b b4 24 30 01 00 	mov    0x130(%rsp),%rsi
   29981:	00 
   29982:	48 85 f6             	test   %rsi,%rsi
   29985:	74 1c                	je     299a3 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x2113>
   29987:	48 8b bc 24 38 01 00 	mov    0x138(%rsp),%rdi
   2998e:	00 
   2998f:	48 c1 e6 03          	shl    $0x3,%rsi
   29993:	ba 08 00 00 00       	mov    $0x8,%edx
   29998:	ff 15 42 9d 04 00    	call   *0x49d42(%rip)        # 736e0 <_DYNAMIC+0x250>
   2999e:	eb 03                	jmp    299a3 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x2113>
   299a0:	49 89 c4             	mov    %rax,%r12
   299a3:	48 8b 74 24 78       	mov    0x78(%rsp),%rsi
   299a8:	48 85 f6             	test   %rsi,%rsi
   299ab:	74 17                	je     299c4 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x2134>
   299ad:	48 8b bc 24 80 00 00 	mov    0x80(%rsp),%rdi
   299b4:	00 
   299b5:	48 c1 e6 03          	shl    $0x3,%rsi
   299b9:	ba 08 00 00 00       	mov    $0x8,%edx
   299be:	ff 15 1c 9d 04 00    	call   *0x49d1c(%rip)        # 736e0 <_DYNAMIC+0x250>
   299c4:	be 00 8c 0a 00       	mov    $0xa8c00,%esi
   299c9:	ba 08 00 00 00       	mov    $0x8,%edx
   299ce:	48 8b 7c 24 18       	mov    0x18(%rsp),%rdi
   299d3:	ff 15 07 9d 04 00    	call   *0x49d07(%rip)        # 736e0 <_DYNAMIC+0x250>
   299d9:	4c 89 e7             	mov    %r12,%rdi
   299dc:	e8 5f 5d 04 00       	call   6f740 <_Unwind_Resume@plt>
