
/tmp/xdremux-style-fit-accumulation-v2-target/candidate/release/xdremux-style-fit-probe:     file format elf64-x86-64


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
   278af:	0f 80 cb 17 00 00    	jo     29080 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x17f0>
   278b5:	48 89 8c 24 70 01 00 	mov    %rcx,0x170(%rsp)
   278bc:	00 
   278bd:	b9 03 00 00 00       	mov    $0x3,%ecx
   278c2:	48 f7 e1             	mul    %rcx
   278c5:	0f 80 b5 17 00 00    	jo     29080 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x17f0>
   278cb:	4d 85 f6             	test   %r14,%r14
   278ce:	0f 84 ac 17 00 00    	je     29080 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x17f0>
   278d4:	48 83 bc 24 70 01 00 	cmpq   $0x0,0x170(%rsp)
   278db:	00 00 
   278dd:	0f 84 9d 17 00 00    	je     29080 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x17f0>
   278e3:	49 89 c7             	mov    %rax,%r15
   278e6:	48 39 46 08          	cmp    %rax,0x8(%rsi)
   278ea:	0f 85 90 17 00 00    	jne    29080 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x17f0>
   278f0:	4c 39 7e 18          	cmp    %r15,0x18(%rsi)
   278f4:	0f 85 86 17 00 00    	jne    29080 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x17f0>
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
   2791c:	0f 87 5e 17 00 00    	ja     29080 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x17f0>
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
   2794b:	0f 87 2f 17 00 00    	ja     29080 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x17f0>
   27951:	48 83 c1 04          	add    $0x4,%rcx
   27955:	48 39 c8             	cmp    %rcx,%rax
   27958:	75 e6                	jne    27940 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0xb0>
   2795a:	48 89 bc 24 10 01 00 	mov    %rdi,0x110(%rsp)
   27961:	00 
   27962:	ff 15 a0 bd 04 00    	call   *0x4bda0(%rip)        # 73708 <_DYNAMIC+0x288>
   27968:	bf 00 8c 0a 00       	mov    $0xa8c00,%edi
   2796d:	be 08 00 00 00       	mov    $0x8,%esi
   27972:	ff 15 d0 be 04 00    	call   *0x4bed0(%rip)        # 73848 <_DYNAMIC+0x3c8>
   27978:	48 89 44 24 18       	mov    %rax,0x18(%rsp)
   2797d:	48 85 c0             	test   %rax,%rax
   27980:	0f 84 2a 1f 00 00    	je     298b0 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x2020>
   27986:	ff 15 7c bd 04 00    	call   *0x4bd7c(%rip)        # 73708 <_DYNAMIC+0x288>
   2798c:	bf 00 2a 03 00       	mov    $0x32a00,%edi
   27991:	be 08 00 00 00       	mov    $0x8,%esi
   27996:	ff 15 ac be 04 00    	call   *0x4beac(%rip)        # 73848 <_DYNAMIC+0x3c8>
   2799c:	48 85 c0             	test   %rax,%rax
   2799f:	0f 84 1b 1f 00 00    	je     298c0 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x2030>
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
   279de:	48 8d 05 63 97 04 00 	lea    0x49763(%rip),%rax        # 71148 <anon.e11926f7acc299ef6543ba90fd675eca.18.llvm.1662880743899104407+0x1f8>
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
   27adb:	0f 84 96 0b 00 00    	je     28677 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0xde7>
   27ae1:	4a 8d 04 29          	lea    (%rcx,%r13,1),%rax
   27ae5:	48 8d 3c 40          	lea    (%rax,%rax,2),%rdi
   27ae9:	4c 39 ff             	cmp    %r15,%rdi
   27aec:	0f 83 86 1d 00 00    	jae    29878 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1fe8>
   27af2:	48 8d 47 01          	lea    0x1(%rdi),%rax
   27af6:	4c 39 f8             	cmp    %r15,%rax
   27af9:	0f 83 5b 1d 00 00    	jae    2985a <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1fca>
   27aff:	4c 89 fa             	mov    %r15,%rdx
   27b02:	48 89 4c 24 70       	mov    %rcx,0x70(%rsp)
   27b07:	48 8d 47 02          	lea    0x2(%rdi),%rax
   27b0b:	4c 39 f8             	cmp    %r15,%rax
   27b0e:	0f 83 52 1d 00 00    	jae    29866 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1fd6>
   27b14:	49 89 d7             	mov    %rdx,%r15
   27b17:	f3 41 0f 10 04 bc    	movss  (%r12,%rdi,4),%xmm0
   27b1d:	0f 57 e4             	xorps  %xmm4,%xmm4
   27b20:	f3 0f 5a e0          	cvtss2sd %xmm0,%xmm4
   27b24:	f3 0f 10 44 bb 08    	movss  0x8(%rbx,%rdi,4),%xmm0
   27b2a:	f3 0f 11 84 24 78 01 	movss  %xmm0,0x178(%rsp)
   27b31:	00 00 
   27b33:	0f 5a 04 bb          	cvtps2pd (%rbx,%rdi,4),%xmm0
   27b37:	0f 29 04 24          	movaps %xmm0,(%rsp)
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
   27c73:	4c 8b 35 16 be 04 00 	mov    0x4be16(%rip),%r14        # 73a90 <exp@GLIBC_2.29>
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
   27e6c:	f2 0f 10 54 24 30    	movsd  0x30(%rsp),%xmm2
   27e72:	48 8b 44 24 28       	mov    0x28(%rsp),%rax
   27e77:	48 89 84 24 90 00 00 	mov    %rax,0x90(%rsp)
   27e7e:	00 
   27e7f:	48 8d 0c c0          	lea    (%rax,%rax,8),%rcx
   27e83:	66 0f 28 9c 24 f0 01 	movapd 0x1f0(%rsp),%xmm3
   27e8a:	00 00 
   27e8c:	66 0f 14 9c 24 d0 01 	unpcklpd 0x1d0(%rsp),%xmm3
   27e93:	00 00 
   27e95:	66 0f 28 0c 24       	movapd (%rsp),%xmm1
   27e9a:	66 0f 5c cb          	subpd  %xmm3,%xmm1
   27e9e:	66 0f 29 0c 24       	movapd %xmm1,(%rsp)
   27ea3:	f2 0f 5c 84 24 e0 01 	subsd  0x1e0(%rsp),%xmm0
   27eaa:	00 00 
   27eac:	f2 0f 10 4c 24 40    	movsd  0x40(%rsp),%xmm1
   27eb2:	48 8b 44 24 38       	mov    0x38(%rsp),%rax
   27eb7:	48 89 44 24 50       	mov    %rax,0x50(%rsp)
   27ebc:	66 0f 28 da          	movapd %xmm2,%xmm3
   27ec0:	f2 0f 59 9c 24 20 01 	mulsd  0x120(%rsp),%xmm3
   27ec7:	00 00 
   27ec9:	66 0f 57 e4          	xorpd  %xmm4,%xmm4
   27ecd:	66 0f 2e e3          	ucomisd %xmm3,%xmm4
   27ed1:	0f 83 d9 01 00 00    	jae    280b0 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x820>
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
   27f21:	0f 84 89 01 00 00    	je     280b0 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x820>
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
   27f54:	0f 87 3e 11 00 00    	ja     29098 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1808>
   27f5a:	48 8d 14 3f          	lea    (%rdi,%rdi,1),%rdx
   27f5e:	48 c1 e7 05          	shl    $0x5,%rdi
   27f62:	48 29 d7             	sub    %rdx,%rdi
   27f65:	48 8d 77 1e          	lea    0x1e(%rdi),%rsi
   27f69:	48 8b 94 24 88 00 00 	mov    0x88(%rsp),%rdx
   27f70:	00 
   27f71:	48 83 ff e1          	cmp    $0xffffffffffffffe1,%rdi
   27f75:	0f 87 32 11 00 00    	ja     290ad <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x181d>
   27f7b:	48 39 d6             	cmp    %rdx,%rsi
   27f7e:	0f 87 29 11 00 00    	ja     290ad <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x181d>
   27f84:	48 8b 54 24 18       	mov    0x18(%rsp),%rdx
   27f89:	4a 8d 14 f2          	lea    (%rdx,%r14,8),%rdx
   27f8d:	48 c1 e7 03          	shl    $0x3,%rdi
   27f91:	48 03 bc 24 80 00 00 	add    0x80(%rsp),%rdi
   27f98:	00 
   27f99:	41 bf 0a 00 00 00    	mov    $0xa,%r15d
   27f9f:	4c 8d a4 24 b8 00 00 	lea    0xb8(%rsp),%r12
   27fa6:	00 
   27fa7:	49 89 c2             	mov    %rax,%r10
   27faa:	45 31 f6             	xor    %r14d,%r14d
   27fad:	eb 7c                	jmp    2802b <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x79b>
   27faf:	90                   	nop
   27fb0:	45 31 c9             	xor    %r9d,%r9d
   27fb3:	4b 8d 34 b6          	lea    (%r14,%r14,4),%rsi
   27fb7:	48 c1 e6 04          	shl    $0x4,%rsi
   27fbb:	48 01 d6             	add    %rdx,%rsi
   27fbe:	4e 8d 2c f4          	lea    (%rsp,%r14,8),%r13
   27fc2:	49 81 c5 b0 00 00 00 	add    $0xb0,%r13
   27fc9:	4a 8d 34 f6          	lea    (%rsi,%r14,8),%rsi
   27fcd:	f2 43 0f 10 74 cd 00 	movsd  0x0(%r13,%r9,8),%xmm6
   27fd4:	f2 0f 59 f5          	mulsd  %xmm5,%xmm6
   27fd8:	f2 42 0f 58 34 ce    	addsd  (%rsi,%r9,8),%xmm6
   27fde:	f2 42 0f 11 34 ce    	movsd  %xmm6,(%rsi,%r9,8)
   27fe4:	4b 8d 34 76          	lea    (%r14,%r14,2),%rsi
   27fe8:	49 ff c6             	inc    %r14
   27feb:	66 0f 28 f0          	movapd %xmm0,%xmm6
   27fef:	f2 0f 59 f5          	mulsd  %xmm5,%xmm6
   27ff3:	66 0f 14 ed          	unpcklpd %xmm5,%xmm5
   27ff7:	66 0f 59 2c 24       	mulpd  (%rsp),%xmm5
   27ffc:	66 0f 10 3c f7       	movupd (%rdi,%rsi,8),%xmm7
   28001:	66 0f 58 fd          	addpd  %xmm5,%xmm7
   28005:	66 0f 11 3c f7       	movupd %xmm7,(%rdi,%rsi,8)
   2800a:	f2 0f 58 74 f7 10    	addsd  0x10(%rdi,%rsi,8),%xmm6
   28010:	f2 0f 11 74 f7 10    	movsd  %xmm6,0x10(%rdi,%rsi,8)
   28016:	49 83 c2 58          	add    $0x58,%r10
   2801a:	49 83 c4 08          	add    $0x8,%r12
   2801e:	49 ff cf             	dec    %r15
   28021:	49 83 fe 0a          	cmp    $0xa,%r14
   28025:	0f 84 e5 fe ff ff    	je     27f10 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x680>
   2802b:	f2 42 0f 10 ac f4 b0 	movsd  0xb0(%rsp,%r14,8),%xmm5
   28032:	00 00 00 
   28035:	f2 0f 59 ec          	mulsd  %xmm4,%xmm5
   28039:	49 83 fe 09          	cmp    $0x9,%r14
   2803d:	0f 84 6d ff ff ff    	je     27fb0 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x720>
   28043:	4c 89 fe             	mov    %r15,%rsi
   28046:	48 83 e6 fe          	and    $0xfffffffffffffffe,%rsi
   2804a:	bd 0a 00 00 00       	mov    $0xa,%ebp
   2804f:	4c 29 f5             	sub    %r14,%rbp
   28052:	45 31 c9             	xor    %r9d,%r9d
   28055:	66 66 2e 0f 1f 84 00 	data16 cs nopw 0x0(%rax,%rax,1)
   2805c:	00 00 00 00 
   28060:	f2 43 0f 10 74 cc f8 	movsd  -0x8(%r12,%r9,8),%xmm6
   28067:	f2 0f 59 f5          	mulsd  %xmm5,%xmm6
   2806b:	f2 43 0f 58 74 ca f8 	addsd  -0x8(%r10,%r9,8),%xmm6
   28072:	f2 43 0f 11 74 ca f8 	movsd  %xmm6,-0x8(%r10,%r9,8)
   28079:	f2 43 0f 10 34 cc    	movsd  (%r12,%r9,8),%xmm6
   2807f:	f2 0f 59 f5          	mulsd  %xmm5,%xmm6
   28083:	f2 43 0f 58 34 ca    	addsd  (%r10,%r9,8),%xmm6
   28089:	f2 43 0f 11 34 ca    	movsd  %xmm6,(%r10,%r9,8)
   2808f:	49 83 c1 02          	add    $0x2,%r9
   28093:	4c 39 ce             	cmp    %r9,%rsi
   28096:	75 c8                	jne    28060 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x7d0>
   28098:	40 f6 c5 01          	test   $0x1,%bpl
   2809c:	0f 85 11 ff ff ff    	jne    27fb3 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x723>
   280a2:	e9 3d ff ff ff       	jmp    27fe4 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x754>
   280a7:	66 0f 1f 84 00 00 00 	nopw   0x0(%rax,%rax,1)
   280ae:	00 00 
   280b0:	48 8b 44 24 50       	mov    0x50(%rsp),%rax
   280b5:	4c 8d 04 c0          	lea    (%rax,%rax,8),%r8
   280b9:	66 0f 28 d9          	movapd %xmm1,%xmm3
   280bd:	f2 0f 59 9c 24 20 01 	mulsd  0x120(%rsp),%xmm3
   280c4:	00 00 
   280c6:	66 0f 57 e4          	xorpd  %xmm4,%xmm4
   280ca:	66 0f 2e e3          	ucomisd %xmm3,%xmm4
   280ce:	0f 83 cc 01 00 00    	jae    282a0 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0xa10>
   280d4:	48 8b 84 24 c8 01 00 	mov    0x1c8(%rsp),%rax
   280db:	00 
   280dc:	4e 8d 14 00          	lea    (%rax,%r8,1),%r10
   280e0:	49 c1 e2 03          	shl    $0x3,%r10
   280e4:	4c 69 5c 24 50 00 e1 	imul   $0xe100,0x50(%rsp),%r11
   280eb:	00 00 
   280ed:	4c 03 9c 24 c0 01 00 	add    0x1c0(%rsp),%r11
   280f4:	00 
   280f5:	31 db                	xor    %ebx,%ebx
   280f7:	45 31 e4             	xor    %r12d,%r12d
   280fa:	eb 1c                	jmp    28118 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x888>
   280fc:	0f 1f 40 00          	nopl   0x0(%rax)
   28100:	48 83 c3 08          	add    $0x8,%rbx
   28104:	49 ff c4             	inc    %r12
   28107:	49 81 c3 20 03 00 00 	add    $0x320,%r11
   2810e:	48 83 fb 40          	cmp    $0x40,%rbx
   28112:	0f 84 88 01 00 00    	je     282a0 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0xa10>
   28118:	f2 0f 10 a4 1c 30 02 	movsd  0x230(%rsp,%rbx,1),%xmm4
   2811f:	00 00 
   28121:	f2 0f 59 e3          	mulsd  %xmm3,%xmm4
   28125:	f2 0f 5e 64 24 60    	divsd  0x60(%rsp),%xmm4
   2812b:	66 0f 57 ed          	xorpd  %xmm5,%xmm5
   2812f:	66 0f 2e ec          	ucomisd %xmm4,%xmm5
   28133:	73 cb                	jae    28100 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x870>
   28135:	4b 8d 3c 14          	lea    (%r12,%r10,1),%rdi
   28139:	4c 6b f7 64          	imul   $0x64,%rdi,%r14
   2813d:	49 81 fe 1c 51 01 00 	cmp    $0x1511c,%r14
   28144:	0f 87 4e 0f 00 00    	ja     29098 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1808>
   2814a:	48 8d 04 3f          	lea    (%rdi,%rdi,1),%rax
   2814e:	48 c1 e7 05          	shl    $0x5,%rdi
   28152:	48 29 c7             	sub    %rax,%rdi
   28155:	48 8d 77 1e          	lea    0x1e(%rdi),%rsi
   28159:	48 8b 94 24 88 00 00 	mov    0x88(%rsp),%rdx
   28160:	00 
   28161:	48 83 ff e1          	cmp    $0xffffffffffffffe1,%rdi
   28165:	0f 87 42 0f 00 00    	ja     290ad <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x181d>
   2816b:	48 39 d6             	cmp    %rdx,%rsi
   2816e:	0f 87 39 0f 00 00    	ja     290ad <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x181d>
   28174:	48 8b 44 24 18       	mov    0x18(%rsp),%rax
   28179:	4a 8d 14 f0          	lea    (%rax,%r14,8),%rdx
   2817d:	48 c1 e7 03          	shl    $0x3,%rdi
   28181:	48 03 bc 24 80 00 00 	add    0x80(%rsp),%rdi
   28188:	00 
   28189:	41 bd 0a 00 00 00    	mov    $0xa,%r13d
   2818f:	48 8d 84 24 b8 00 00 	lea    0xb8(%rsp),%rax
   28196:	00 
   28197:	4d 89 de             	mov    %r11,%r14
   2819a:	45 31 c9             	xor    %r9d,%r9d
   2819d:	eb 78                	jmp    28217 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x987>
   2819f:	90                   	nop
   281a0:	31 ed                	xor    %ebp,%ebp
   281a2:	4b 8d 34 89          	lea    (%r9,%r9,4),%rsi
   281a6:	48 c1 e6 04          	shl    $0x4,%rsi
   281aa:	48 01 d6             	add    %rdx,%rsi
   281ad:	4e 8d 3c cc          	lea    (%rsp,%r9,8),%r15
   281b1:	49 81 c7 b0 00 00 00 	add    $0xb0,%r15
   281b8:	4a 8d 34 ce          	lea    (%rsi,%r9,8),%rsi
   281bc:	f2 41 0f 10 34 ef    	movsd  (%r15,%rbp,8),%xmm6
   281c2:	f2 0f 59 f5          	mulsd  %xmm5,%xmm6
   281c6:	f2 0f 58 34 ee       	addsd  (%rsi,%rbp,8),%xmm6
   281cb:	f2 0f 11 34 ee       	movsd  %xmm6,(%rsi,%rbp,8)
   281d0:	4b 8d 34 49          	lea    (%r9,%r9,2),%rsi
   281d4:	49 ff c1             	inc    %r9
   281d7:	66 0f 28 f0          	movapd %xmm0,%xmm6
   281db:	f2 0f 59 f5          	mulsd  %xmm5,%xmm6
   281df:	66 0f 14 ed          	unpcklpd %xmm5,%xmm5
   281e3:	66 0f 59 2c 24       	mulpd  (%rsp),%xmm5
   281e8:	66 0f 10 3c f7       	movupd (%rdi,%rsi,8),%xmm7
   281ed:	66 0f 58 fd          	addpd  %xmm5,%xmm7
   281f1:	66 0f 11 3c f7       	movupd %xmm7,(%rdi,%rsi,8)
   281f6:	f2 0f 58 74 f7 10    	addsd  0x10(%rdi,%rsi,8),%xmm6
   281fc:	f2 0f 11 74 f7 10    	movsd  %xmm6,0x10(%rdi,%rsi,8)
   28202:	49 83 c6 58          	add    $0x58,%r14
   28206:	48 83 c0 08          	add    $0x8,%rax
   2820a:	49 ff cd             	dec    %r13
   2820d:	49 83 f9 0a          	cmp    $0xa,%r9
   28211:	0f 84 e9 fe ff ff    	je     28100 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x870>
   28217:	f2 42 0f 10 ac cc b0 	movsd  0xb0(%rsp,%r9,8),%xmm5
   2821e:	00 00 00 
   28221:	f2 0f 59 ec          	mulsd  %xmm4,%xmm5
   28225:	49 83 f9 09          	cmp    $0x9,%r9
   28229:	0f 84 71 ff ff ff    	je     281a0 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x910>
   2822f:	4c 89 ee             	mov    %r13,%rsi
   28232:	48 83 e6 fe          	and    $0xfffffffffffffffe,%rsi
   28236:	41 bf 0a 00 00 00    	mov    $0xa,%r15d
   2823c:	4d 29 cf             	sub    %r9,%r15
   2823f:	31 ed                	xor    %ebp,%ebp
   28241:	66 66 66 66 66 66 2e 	data16 data16 data16 data16 data16 cs nopw 0x0(%rax,%rax,1)
   28248:	0f 1f 84 00 00 00 00 
   2824f:	00 
   28250:	f2 0f 10 74 e8 f8    	movsd  -0x8(%rax,%rbp,8),%xmm6
   28256:	f2 0f 59 f5          	mulsd  %xmm5,%xmm6
   2825a:	f2 41 0f 58 74 ee f8 	addsd  -0x8(%r14,%rbp,8),%xmm6
   28261:	f2 41 0f 11 74 ee f8 	movsd  %xmm6,-0x8(%r14,%rbp,8)
   28268:	f2 0f 10 34 e8       	movsd  (%rax,%rbp,8),%xmm6
   2826d:	f2 0f 59 f5          	mulsd  %xmm5,%xmm6
   28271:	f2 41 0f 58 34 ee    	addsd  (%r14,%rbp,8),%xmm6
   28277:	f2 41 0f 11 34 ee    	movsd  %xmm6,(%r14,%rbp,8)
   2827d:	48 83 c5 02          	add    $0x2,%rbp
   28281:	48 39 ee             	cmp    %rbp,%rsi
   28284:	75 ca                	jne    28250 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x9c0>
   28286:	41 f6 c7 01          	test   $0x1,%r15b
   2828a:	0f 85 12 ff ff ff    	jne    281a2 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x912>
   28290:	e9 3b ff ff ff       	jmp    281d0 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x940>
   28295:	66 66 2e 0f 1f 84 00 	data16 cs nopw 0x0(%rax,%rax,1)
   2829c:	00 00 00 00 
   282a0:	f2 0f 59 94 24 18 01 	mulsd  0x118(%rsp),%xmm2
   282a7:	00 00 
   282a9:	66 0f 57 db          	xorpd  %xmm3,%xmm3
   282ad:	66 0f 2e da          	ucomisd %xmm2,%xmm3
   282b1:	48 8d ac 24 30 01 00 	lea    0x130(%rsp),%rbp
   282b8:	00 
   282b9:	4c 8b 6c 24 20       	mov    0x20(%rsp),%r13
   282be:	0f 83 dc 01 00 00    	jae    284a0 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0xc10>
   282c4:	48 03 8c 24 b8 01 00 	add    0x1b8(%rsp),%rcx
   282cb:	00 
   282cc:	48 c1 e1 03          	shl    $0x3,%rcx
   282d0:	4c 69 8c 24 90 00 00 	imul   $0xe100,0x90(%rsp),%r9
   282d7:	00 00 e1 00 00 
   282dc:	4c 03 8c 24 b0 01 00 	add    0x1b0(%rsp),%r9
   282e3:	00 
   282e4:	45 31 d2             	xor    %r10d,%r10d
   282e7:	45 31 db             	xor    %r11d,%r11d
   282ea:	eb 1c                	jmp    28308 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0xa78>
   282ec:	0f 1f 40 00          	nopl   0x0(%rax)
   282f0:	49 83 c2 08          	add    $0x8,%r10
   282f4:	49 ff c3             	inc    %r11
   282f7:	49 81 c1 20 03 00 00 	add    $0x320,%r9
   282fe:	49 83 fa 40          	cmp    $0x40,%r10
   28302:	0f 84 98 01 00 00    	je     284a0 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0xc10>
   28308:	f2 42 0f 10 9c 14 30 	movsd  0x230(%rsp,%r10,1),%xmm3
   2830f:	02 00 00 
   28312:	f2 0f 59 da          	mulsd  %xmm2,%xmm3
   28316:	f2 0f 5e 5c 24 60    	divsd  0x60(%rsp),%xmm3
   2831c:	66 0f 57 e4          	xorpd  %xmm4,%xmm4
   28320:	66 0f 2e e3          	ucomisd %xmm3,%xmm4
   28324:	73 ca                	jae    282f0 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0xa60>
   28326:	49 8d 3c 0b          	lea    (%r11,%rcx,1),%rdi
   2832a:	4c 6b f7 64          	imul   $0x64,%rdi,%r14
   2832e:	49 81 fe 1c 51 01 00 	cmp    $0x1511c,%r14
   28335:	0f 87 5d 0d 00 00    	ja     29098 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1808>
   2833b:	48 8d 04 3f          	lea    (%rdi,%rdi,1),%rax
   2833f:	48 c1 e7 05          	shl    $0x5,%rdi
   28343:	48 29 c7             	sub    %rax,%rdi
   28346:	48 8d 77 1e          	lea    0x1e(%rdi),%rsi
   2834a:	48 8b 94 24 88 00 00 	mov    0x88(%rsp),%rdx
   28351:	00 
   28352:	48 83 ff e1          	cmp    $0xffffffffffffffe1,%rdi
   28356:	0f 87 51 0d 00 00    	ja     290ad <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x181d>
   2835c:	48 39 d6             	cmp    %rdx,%rsi
   2835f:	0f 87 48 0d 00 00    	ja     290ad <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x181d>
   28365:	48 8b 44 24 18       	mov    0x18(%rsp),%rax
   2836a:	4a 8d 14 f0          	lea    (%rax,%r14,8),%rdx
   2836e:	48 c1 e7 03          	shl    $0x3,%rdi
   28372:	48 03 bc 24 80 00 00 	add    0x80(%rsp),%rdi
   28379:	00 
   2837a:	be 0a 00 00 00       	mov    $0xa,%esi
   2837f:	48 8d 84 24 b8 00 00 	lea    0xb8(%rsp),%rax
   28386:	00 
   28387:	4c 89 cb             	mov    %r9,%rbx
   2838a:	45 31 f6             	xor    %r14d,%r14d
   2838d:	e9 92 00 00 00       	jmp    28424 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0xb94>
   28392:	66 66 66 66 66 2e 0f 	data16 data16 data16 data16 cs nopw 0x0(%rax,%rax,1)
   28399:	1f 84 00 00 00 00 00 
   283a0:	45 31 e4             	xor    %r12d,%r12d
   283a3:	4f 8d 3c b6          	lea    (%r14,%r14,4),%r15
   283a7:	49 c1 e7 04          	shl    $0x4,%r15
   283ab:	49 01 d7             	add    %rdx,%r15
   283ae:	4e 8d 2c f4          	lea    (%rsp,%r14,8),%r13
   283b2:	49 81 c5 b0 00 00 00 	add    $0xb0,%r13
   283b9:	4f 8d 3c f7          	lea    (%r15,%r14,8),%r15
   283bd:	f2 43 0f 10 6c e5 00 	movsd  0x0(%r13,%r12,8),%xmm5
   283c4:	4c 8b 6c 24 20       	mov    0x20(%rsp),%r13
   283c9:	f2 0f 59 ec          	mulsd  %xmm4,%xmm5
   283cd:	f2 43 0f 58 2c e7    	addsd  (%r15,%r12,8),%xmm5
   283d3:	f2 43 0f 11 2c e7    	movsd  %xmm5,(%r15,%r12,8)
   283d9:	4f 8d 3c 76          	lea    (%r14,%r14,2),%r15
   283dd:	49 ff c6             	inc    %r14
   283e0:	66 0f 28 e8          	movapd %xmm0,%xmm5
   283e4:	f2 0f 59 ec          	mulsd  %xmm4,%xmm5
   283e8:	66 0f 14 e4          	unpcklpd %xmm4,%xmm4
   283ec:	66 0f 59 24 24       	mulpd  (%rsp),%xmm4
   283f1:	66 42 0f 10 34 ff    	movupd (%rdi,%r15,8),%xmm6
   283f7:	66 0f 58 f4          	addpd  %xmm4,%xmm6
   283fb:	66 42 0f 11 34 ff    	movupd %xmm6,(%rdi,%r15,8)
   28401:	f2 42 0f 58 6c ff 10 	addsd  0x10(%rdi,%r15,8),%xmm5
   28408:	f2 42 0f 11 6c ff 10 	movsd  %xmm5,0x10(%rdi,%r15,8)
   2840f:	48 83 c3 58          	add    $0x58,%rbx
   28413:	48 83 c0 08          	add    $0x8,%rax
   28417:	48 ff ce             	dec    %rsi
   2841a:	49 83 fe 0a          	cmp    $0xa,%r14
   2841e:	0f 84 cc fe ff ff    	je     282f0 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0xa60>
   28424:	f2 42 0f 10 a4 f4 b0 	movsd  0xb0(%rsp,%r14,8),%xmm4
   2842b:	00 00 00 
   2842e:	f2 0f 59 e3          	mulsd  %xmm3,%xmm4
   28432:	49 83 fe 09          	cmp    $0x9,%r14
   28436:	0f 84 64 ff ff ff    	je     283a0 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0xb10>
   2843c:	48 89 f5             	mov    %rsi,%rbp
   2843f:	48 83 e5 fe          	and    $0xfffffffffffffffe,%rbp
   28443:	41 bf 0a 00 00 00    	mov    $0xa,%r15d
   28449:	4d 29 f7             	sub    %r14,%r15
   2844c:	45 31 e4             	xor    %r12d,%r12d
   2844f:	90                   	nop
   28450:	f2 42 0f 10 6c e0 f8 	movsd  -0x8(%rax,%r12,8),%xmm5
   28457:	f2 0f 59 ec          	mulsd  %xmm4,%xmm5
   2845b:	f2 42 0f 58 6c e3 f8 	addsd  -0x8(%rbx,%r12,8),%xmm5
   28462:	f2 42 0f 11 6c e3 f8 	movsd  %xmm5,-0x8(%rbx,%r12,8)
   28469:	f2 42 0f 10 2c e0    	movsd  (%rax,%r12,8),%xmm5
   2846f:	f2 0f 59 ec          	mulsd  %xmm4,%xmm5
   28473:	f2 42 0f 58 2c e3    	addsd  (%rbx,%r12,8),%xmm5
   28479:	f2 42 0f 11 2c e3    	movsd  %xmm5,(%rbx,%r12,8)
   2847f:	49 83 c4 02          	add    $0x2,%r12
   28483:	4c 39 e5             	cmp    %r12,%rbp
   28486:	75 c8                	jne    28450 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0xbc0>
   28488:	41 f6 c7 01          	test   $0x1,%r15b
   2848c:	48 8d ac 24 30 01 00 	lea    0x130(%rsp),%rbp
   28493:	00 
   28494:	0f 85 09 ff ff ff    	jne    283a3 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0xb13>
   2849a:	e9 3a ff ff ff       	jmp    283d9 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0xb49>
   2849f:	90                   	nop
   284a0:	f2 0f 59 8c 24 18 01 	mulsd  0x118(%rsp),%xmm1
   284a7:	00 00 
   284a9:	66 0f 57 d2          	xorpd  %xmm2,%xmm2
   284ad:	66 0f 2e d1          	ucomisd %xmm1,%xmm2
   284b1:	0f 83 f9 f5 ff ff    	jae    27ab0 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x220>
   284b7:	4c 03 84 24 b8 01 00 	add    0x1b8(%rsp),%r8
   284be:	00 
   284bf:	49 c1 e0 03          	shl    $0x3,%r8
   284c3:	48 69 44 24 50 00 e1 	imul   $0xe100,0x50(%rsp),%rax
   284ca:	00 00 
   284cc:	48 03 84 24 b0 01 00 	add    0x1b0(%rsp),%rax
   284d3:	00 
   284d4:	31 c9                	xor    %ecx,%ecx
   284d6:	45 31 c9             	xor    %r9d,%r9d
   284d9:	eb 1c                	jmp    284f7 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0xc67>
   284db:	0f 1f 44 00 00       	nopl   0x0(%rax,%rax,1)
   284e0:	48 83 c1 08          	add    $0x8,%rcx
   284e4:	49 ff c1             	inc    %r9
   284e7:	48 05 20 03 00 00    	add    $0x320,%rax
   284ed:	48 83 f9 40          	cmp    $0x40,%rcx
   284f1:	0f 84 b9 f5 ff ff    	je     27ab0 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x220>
   284f7:	f2 0f 10 94 0c 30 02 	movsd  0x230(%rsp,%rcx,1),%xmm2
   284fe:	00 00 
   28500:	f2 0f 59 d1          	mulsd  %xmm1,%xmm2
   28504:	f2 0f 5e 54 24 60    	divsd  0x60(%rsp),%xmm2
   2850a:	66 0f 57 db          	xorpd  %xmm3,%xmm3
   2850e:	66 0f 2e da          	ucomisd %xmm2,%xmm3
   28512:	73 cc                	jae    284e0 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0xc50>
   28514:	4b 8d 3c 01          	lea    (%r9,%r8,1),%rdi
   28518:	4c 6b f7 64          	imul   $0x64,%rdi,%r14
   2851c:	49 81 fe 1c 51 01 00 	cmp    $0x1511c,%r14
   28523:	0f 87 6f 0b 00 00    	ja     29098 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1808>
   28529:	48 8d 14 3f          	lea    (%rdi,%rdi,1),%rdx
   2852d:	48 c1 e7 05          	shl    $0x5,%rdi
   28531:	48 29 d7             	sub    %rdx,%rdi
   28534:	48 8d 77 1e          	lea    0x1e(%rdi),%rsi
   28538:	48 8b 94 24 88 00 00 	mov    0x88(%rsp),%rdx
   2853f:	00 
   28540:	48 83 ff e1          	cmp    $0xffffffffffffffe1,%rdi
   28544:	0f 87 63 0b 00 00    	ja     290ad <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x181d>
   2854a:	48 39 d6             	cmp    %rdx,%rsi
   2854d:	0f 87 5a 0b 00 00    	ja     290ad <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x181d>
   28553:	48 8b 54 24 18       	mov    0x18(%rsp),%rdx
   28558:	4a 8d 14 f2          	lea    (%rdx,%r14,8),%rdx
   2855c:	48 c1 e7 03          	shl    $0x3,%rdi
   28560:	48 03 bc 24 80 00 00 	add    0x80(%rsp),%rdi
   28567:	00 
   28568:	be 0a 00 00 00       	mov    $0xa,%esi
   2856d:	4c 8d 94 24 b8 00 00 	lea    0xb8(%rsp),%r10
   28574:	00 
   28575:	49 89 c3             	mov    %rax,%r11
   28578:	31 db                	xor    %ebx,%ebx
   2857a:	e9 7f 00 00 00       	jmp    285fe <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0xd6e>
   2857f:	90                   	nop
   28580:	45 31 f6             	xor    %r14d,%r14d
   28583:	4c 8d 3c 9b          	lea    (%rbx,%rbx,4),%r15
   28587:	49 c1 e7 04          	shl    $0x4,%r15
   2858b:	49 01 d7             	add    %rdx,%r15
   2858e:	4c 8d 24 dc          	lea    (%rsp,%rbx,8),%r12
   28592:	49 81 c4 b0 00 00 00 	add    $0xb0,%r12
   28599:	4d 8d 3c df          	lea    (%r15,%rbx,8),%r15
   2859d:	f2 43 0f 10 24 f4    	movsd  (%r12,%r14,8),%xmm4
   285a3:	f2 0f 59 e3          	mulsd  %xmm3,%xmm4
   285a7:	f2 43 0f 58 24 f7    	addsd  (%r15,%r14,8),%xmm4
   285ad:	f2 43 0f 11 24 f7    	movsd  %xmm4,(%r15,%r14,8)
   285b3:	4c 8d 34 5b          	lea    (%rbx,%rbx,2),%r14
   285b7:	48 ff c3             	inc    %rbx
   285ba:	66 0f 28 e0          	movapd %xmm0,%xmm4
   285be:	f2 0f 59 e3          	mulsd  %xmm3,%xmm4
   285c2:	66 0f 14 db          	unpcklpd %xmm3,%xmm3
   285c6:	66 0f 59 1c 24       	mulpd  (%rsp),%xmm3
   285cb:	66 42 0f 10 2c f7    	movupd (%rdi,%r14,8),%xmm5
   285d1:	66 0f 58 eb          	addpd  %xmm3,%xmm5
   285d5:	66 42 0f 11 2c f7    	movupd %xmm5,(%rdi,%r14,8)
   285db:	f2 42 0f 58 64 f7 10 	addsd  0x10(%rdi,%r14,8),%xmm4
   285e2:	f2 42 0f 11 64 f7 10 	movsd  %xmm4,0x10(%rdi,%r14,8)
   285e9:	49 83 c3 58          	add    $0x58,%r11
   285ed:	49 83 c2 08          	add    $0x8,%r10
   285f1:	48 ff ce             	dec    %rsi
   285f4:	48 83 fb 0a          	cmp    $0xa,%rbx
   285f8:	0f 84 e2 fe ff ff    	je     284e0 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0xc50>
   285fe:	f2 0f 10 9c dc b0 00 	movsd  0xb0(%rsp,%rbx,8),%xmm3
   28605:	00 00 
   28607:	f2 0f 59 da          	mulsd  %xmm2,%xmm3
   2860b:	48 83 fb 09          	cmp    $0x9,%rbx
   2860f:	0f 84 6b ff ff ff    	je     28580 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0xcf0>
   28615:	49 89 f4             	mov    %rsi,%r12
   28618:	49 83 e4 fe          	and    $0xfffffffffffffffe,%r12
   2861c:	41 bf 0a 00 00 00    	mov    $0xa,%r15d
   28622:	49 29 df             	sub    %rbx,%r15
   28625:	45 31 f6             	xor    %r14d,%r14d
   28628:	0f 1f 84 00 00 00 00 	nopl   0x0(%rax,%rax,1)
   2862f:	00 
   28630:	f2 43 0f 10 64 f2 f8 	movsd  -0x8(%r10,%r14,8),%xmm4
   28637:	f2 0f 59 e3          	mulsd  %xmm3,%xmm4
   2863b:	f2 43 0f 58 64 f3 f8 	addsd  -0x8(%r11,%r14,8),%xmm4
   28642:	f2 43 0f 11 64 f3 f8 	movsd  %xmm4,-0x8(%r11,%r14,8)
   28649:	f2 43 0f 10 24 f2    	movsd  (%r10,%r14,8),%xmm4
   2864f:	f2 0f 59 e3          	mulsd  %xmm3,%xmm4
   28653:	f2 43 0f 58 24 f3    	addsd  (%r11,%r14,8),%xmm4
   28659:	f2 43 0f 11 24 f3    	movsd  %xmm4,(%r11,%r14,8)
   2865f:	49 83 c6 02          	add    $0x2,%r14
   28663:	4d 39 f4             	cmp    %r14,%r12
   28666:	75 c8                	jne    28630 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0xda0>
   28668:	41 f6 c7 01          	test   $0x1,%r15b
   2866c:	0f 85 11 ff ff ff    	jne    28583 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0xcf3>
   28672:	e9 3c ff ff ff       	jmp    285b3 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0xd23>
   28677:	48 8b 84 24 10 02 00 	mov    0x210(%rsp),%rax
   2867e:	00 
   2867f:	49 89 c5             	mov    %rax,%r13
   28682:	48 8b 94 24 70 01 00 	mov    0x170(%rsp),%rdx
   28689:	00 
   2868a:	48 39 d0             	cmp    %rdx,%rax
   2868d:	0f 85 82 f3 ff ff    	jne    27a15 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x185>
   28693:	48 8d bc 24 30 01 00 	lea    0x130(%rsp),%rdi
   2869a:	00 
   2869b:	be 40 65 00 00       	mov    $0x6540,%esi
   286a0:	e8 4b df ff ff       	call   265f0 <<f64 as alloc::vec::spec_from_elem::SpecFromElem>::from_elem::<alloc::alloc::Global>>
   286a5:	b8 80 51 01 00       	mov    $0x15180,%eax
   286aa:	31 c9                	xor    %ecx,%ecx
   286ac:	31 d2                	xor    %edx,%edx
   286ae:	48 8b 74 24 18       	mov    0x18(%rsp),%rsi
   286b3:	31 ff                	xor    %edi,%edi
   286b5:	4c 8d 04 ff          	lea    (%rdi,%rdi,8),%r8
   286b9:	4c 89 84 24 f0 01 00 	mov    %r8,0x1f0(%rsp)
   286c0:	00 
   286c1:	48 ff c7             	inc    %rdi
   286c4:	48 89 bc 24 18 01 00 	mov    %rdi,0x118(%rsp)
   286cb:	00 
   286cc:	48 89 8c 24 78 01 00 	mov    %rcx,0x178(%rsp)
   286d3:	00 
   286d4:	48 89 94 24 d0 01 00 	mov    %rdx,0x1d0(%rsp)
   286db:	00 
   286dc:	48 89 b4 24 20 01 00 	mov    %rsi,0x120(%rsp)
   286e3:	00 
   286e4:	48 89 84 24 e0 01 00 	mov    %rax,0x1e0(%rsp)
   286eb:	00 
   286ec:	31 ff                	xor    %edi,%edi
   286ee:	eb 4d                	jmp    2873d <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0xead>
   286f0:	48 8b 84 24 00 02 00 	mov    0x200(%rsp),%rax
   286f7:	00 
   286f8:	48 05 e0 fc ff ff    	add    $0xfffffffffffffce0,%rax
   286fe:	48 8b b4 24 50 01 00 	mov    0x150(%rsp),%rsi
   28705:	00 
   28706:	48 81 c6 00 19 00 00 	add    $0x1900,%rsi
   2870d:	48 8b 94 24 80 01 00 	mov    0x180(%rsp),%rdx
   28714:	00 
   28715:	48 81 c2 20 03 00 00 	add    $0x320,%rdx
   2871c:	48 8b 8c 24 90 01 00 	mov    0x190(%rsp),%rcx
   28723:	00 
   28724:	48 81 c1 f0 00 00 00 	add    $0xf0,%rcx
   2872b:	48 8b bc 24 28 01 00 	mov    0x128(%rsp),%rdi
   28732:	00 
   28733:	48 83 ff 09          	cmp    $0x9,%rdi
   28737:	0f 84 84 08 00 00    	je     28fc1 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1731>
   2873d:	49 89 f8             	mov    %rdi,%r8
   28740:	48 ff c7             	inc    %rdi
   28743:	48 89 bc 24 28 01 00 	mov    %rdi,0x128(%rsp)
   2874a:	00 
   2874b:	4c 03 84 24 f0 01 00 	add    0x1f0(%rsp),%r8
   28752:	00 
   28753:	49 c1 e0 03          	shl    $0x3,%r8
   28757:	4c 89 44 24 70       	mov    %r8,0x70(%rsp)
   2875c:	48 89 8c 24 90 01 00 	mov    %rcx,0x190(%rsp)
   28763:	00 
   28764:	49 89 cc             	mov    %rcx,%r12
   28767:	48 89 94 24 80 01 00 	mov    %rdx,0x180(%rsp)
   2876e:	00 
   2876f:	48 89 d3             	mov    %rdx,%rbx
   28772:	48 89 b4 24 50 01 00 	mov    %rsi,0x150(%rsp)
   28779:	00 
   2877a:	48 89 74 24 20       	mov    %rsi,0x20(%rsp)
   2877f:	48 89 84 24 00 02 00 	mov    %rax,0x200(%rsp)
   28786:	00 
   28787:	48 89 44 24 60       	mov    %rax,0x60(%rsp)
   2878c:	31 ed                	xor    %ebp,%ebp
   2878e:	eb 43                	jmp    287d3 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0xf43>
   28790:	be 20 03 00 00       	mov    $0x320,%esi
   28795:	ba 08 00 00 00       	mov    $0x8,%edx
   2879a:	4c 89 f7             	mov    %r14,%rdi
   2879d:	ff d3                	call   *%rbx
   2879f:	48 83 44 24 60 9c    	addq   $0xffffffffffffff9c,0x60(%rsp)
   287a5:	48 81 44 24 20 20 03 	addq   $0x320,0x20(%rsp)
   287ac:	00 00 
   287ae:	48 8b 9c 24 90 00 00 	mov    0x90(%rsp),%rbx
   287b5:	00 
   287b6:	48 83 c3 64          	add    $0x64,%rbx
   287ba:	49 83 c4 1e          	add    $0x1e,%r12
   287be:	48 8b 84 24 00 01 00 	mov    0x100(%rsp),%rax
   287c5:	00 
   287c6:	48 89 c5             	mov    %rax,%rbp
   287c9:	48 83 f8 08          	cmp    $0x8,%rax
   287cd:	0f 84 1d ff ff ff    	je     286f0 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0xe60>
   287d3:	ff 15 2f af 04 00    	call   *0x4af2f(%rip)        # 73708 <_DYNAMIC+0x288>
   287d9:	41 bf 20 03 00 00    	mov    $0x320,%r15d
   287df:	41 bd 08 00 00 00    	mov    $0x8,%r13d
   287e5:	bf 20 03 00 00       	mov    $0x320,%edi
   287ea:	be 08 00 00 00       	mov    $0x8,%esi
   287ef:	ff 15 53 b0 04 00    	call   *0x4b053(%rip)        # 73848 <_DYNAMIC+0x3c8>
   287f5:	48 85 c0             	test   %rax,%rax
   287f8:	0f 84 a4 10 00 00    	je     298a2 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x2012>
   287fe:	49 89 c6             	mov    %rax,%r14
   28801:	48 8d 45 01          	lea    0x1(%rbp),%rax
   28805:	48 89 84 24 00 01 00 	mov    %rax,0x100(%rsp)
   2880c:	00 
   2880d:	48 03 6c 24 70       	add    0x70(%rsp),%rbp
   28812:	48 89 6c 24 50       	mov    %rbp,0x50(%rsp)
   28817:	49 8d 46 18          	lea    0x18(%r14),%rax
   2881b:	b9 64 00 00 00       	mov    $0x64,%ecx
   28820:	48 89 9c 24 90 00 00 	mov    %rbx,0x90(%rsp)
   28827:	00 
   28828:	48 89 da             	mov    %rbx,%rdx
   2882b:	48 8b 74 24 60       	mov    0x60(%rsp),%rsi
   28830:	4d 89 f0             	mov    %r14,%r8
   28833:	4c 8b 4c 24 20       	mov    0x20(%rsp),%r9
   28838:	45 31 d2             	xor    %r10d,%r10d
   2883b:	4c 89 e3             	mov    %r12,%rbx
   2883e:	4c 89 34 24          	mov    %r14,(%rsp)
   28842:	66 66 66 66 66 2e 0f 	data16 data16 data16 data16 cs nopw 0x0(%rax,%rax,1)
   28849:	1f 84 00 00 00 00 00 
   28850:	4d 85 d2             	test   %r10,%r10
   28853:	74 1b                	je     28870 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0xfe0>
   28855:	4b 8d 3c 12          	lea    (%r10,%r10,1),%rdi
   28859:	48 8d 3c bf          	lea    (%rdi,%rdi,4),%rdi
   2885d:	49 83 fa 04          	cmp    $0x4,%r10
   28861:	73 1d                	jae    28880 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0xff0>
   28863:	45 31 db             	xor    %r11d,%r11d
   28866:	eb 77                	jmp    288df <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x104f>
   28868:	0f 1f 84 00 00 00 00 	nopl   0x0(%rax,%rax,1)
   2886f:	00 
   28870:	31 ff                	xor    %edi,%edi
   28872:	e9 a1 00 00 00       	jmp    28918 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1088>
   28877:	66 0f 1f 84 00 00 00 	nopw   0x0(%rax,%rax,1)
   2887e:	00 00 
   28880:	4d 89 d7             	mov    %r10,%r15
   28883:	49 83 e7 fc          	and    $0xfffffffffffffffc,%r15
   28887:	4d 89 cd             	mov    %r9,%r13
   2888a:	45 31 db             	xor    %r11d,%r11d
   2888d:	0f 1f 00             	nopl   (%rax)
   28890:	f2 41 0f 10 45 00    	movsd  0x0(%r13),%xmm0
   28896:	f2 42 0f 11 44 d8 e8 	movsd  %xmm0,-0x18(%rax,%r11,8)
   2889d:	f2 41 0f 10 45 50    	movsd  0x50(%r13),%xmm0
   288a3:	f2 42 0f 11 44 d8 f0 	movsd  %xmm0,-0x10(%rax,%r11,8)
   288aa:	f2 41 0f 10 85 a0 00 	movsd  0xa0(%r13),%xmm0
   288b1:	00 00 
   288b3:	f2 42 0f 11 44 d8 f8 	movsd  %xmm0,-0x8(%rax,%r11,8)
   288ba:	f2 41 0f 10 85 f0 00 	movsd  0xf0(%r13),%xmm0
   288c1:	00 00 
   288c3:	f2 42 0f 11 04 d8    	movsd  %xmm0,(%rax,%r11,8)
   288c9:	49 83 c3 04          	add    $0x4,%r11
   288cd:	49 81 c5 40 01 00 00 	add    $0x140,%r13
   288d4:	4d 39 df             	cmp    %r11,%r15
   288d7:	75 b7                	jne    28890 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1000>
   288d9:	41 f6 c2 03          	test   $0x3,%r10b
   288dd:	74 39                	je     28918 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1088>
   288df:	45 89 d7             	mov    %r10d,%r15d
   288e2:	41 83 e7 03          	and    $0x3,%r15d
   288e6:	4f 8d 2c d8          	lea    (%r8,%r11,8),%r13
   288ea:	4f 8d 1c 9b          	lea    (%r11,%r11,4),%r11
   288ee:	49 c1 e3 04          	shl    $0x4,%r11
   288f2:	4d 01 cb             	add    %r9,%r11
   288f5:	45 31 e4             	xor    %r12d,%r12d
   288f8:	0f 1f 84 00 00 00 00 	nopl   0x0(%rax,%rax,1)
   288ff:	00 
   28900:	f2 41 0f 10 03       	movsd  (%r11),%xmm0
   28905:	f2 43 0f 11 44 e5 00 	movsd  %xmm0,0x0(%r13,%r12,8)
   2890c:	49 ff c4             	inc    %r12
   2890f:	49 83 c3 50          	add    $0x50,%r11
   28913:	4d 39 e7             	cmp    %r12,%r15
   28916:	75 e8                	jne    28900 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1070>
   28918:	4d 8d 5a 01          	lea    0x1(%r10),%r11
   2891c:	4e 8d 3c 17          	lea    (%rdi,%r10,1),%r15
   28920:	4f 8d 2c fe          	lea    (%r14,%r15,8),%r13
   28924:	48 01 d7             	add    %rdx,%rdi
   28927:	4c 8b 74 24 18       	mov    0x18(%rsp),%r14
   2892c:	4d 8d 24 fe          	lea    (%r14,%rdi,8),%r12
   28930:	31 ed                	xor    %ebp,%ebp
   28932:	66 66 66 66 66 2e 0f 	data16 data16 data16 data16 cs nopw 0x0(%rax,%rax,1)
   28939:	1f 84 00 00 00 00 00 
   28940:	48 39 ee             	cmp    %rbp,%rsi
   28943:	0f 84 76 07 00 00    	je     290bf <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x182f>
   28949:	48 39 e9             	cmp    %rbp,%rcx
   2894c:	0f 84 7e 07 00 00    	je     290d0 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1840>
   28952:	f2 41 0f 10 04 ec    	movsd  (%r12,%rbp,8),%xmm0
   28958:	f2 41 0f 11 44 ed 00 	movsd  %xmm0,0x0(%r13,%rbp,8)
   2895f:	4d 8d 34 2a          	lea    (%r10,%rbp,1),%r14
   28963:	49 ff c6             	inc    %r14
   28966:	48 ff c5             	inc    %rbp
   28969:	49 83 fe 0a          	cmp    $0xa,%r14
   2896d:	75 d1                	jne    28940 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x10b0>
   2896f:	49 83 c1 08          	add    $0x8,%r9
   28973:	48 83 c0 50          	add    $0x50,%rax
   28977:	49 83 c0 50          	add    $0x50,%r8
   2897b:	48 83 c1 f5          	add    $0xfffffffffffffff5,%rcx
   2897f:	48 83 c6 f5          	add    $0xfffffffffffffff5,%rsi
   28983:	48 ff c2             	inc    %rdx
   28986:	4d 89 da             	mov    %r11,%r10
   28989:	49 83 fb 0a          	cmp    $0xa,%r11
   2898d:	4c 8b 34 24          	mov    (%rsp),%r14
   28991:	0f 85 b9 fe ff ff    	jne    28850 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0xfc0>
   28997:	49 89 dc             	mov    %rbx,%r12
   2899a:	f2 45 0f 10 16       	movsd  (%r14),%xmm10
   2899f:	f2 41 0f 10 7e 58    	movsd  0x58(%r14),%xmm7
   289a5:	66 41 0f 28 c2       	movapd %xmm10,%xmm0
   289aa:	f2 0f 58 c7          	addsd  %xmm7,%xmm0
   289ae:	f2 45 0f 10 86 b0 00 	movsd  0xb0(%r14),%xmm8
   289b5:	00 00 
   289b7:	f2 41 0f 58 c0       	addsd  %xmm8,%xmm0
   289bc:	f2 45 0f 10 8e 08 01 	movsd  0x108(%r14),%xmm9
   289c3:	00 00 
   289c5:	f2 41 0f 58 c1       	addsd  %xmm9,%xmm0
   289ca:	f2 41 0f 10 8e 60 01 	movsd  0x160(%r14),%xmm1
   289d1:	00 00 
   289d3:	f2 0f 58 c1          	addsd  %xmm1,%xmm0
   289d7:	f2 41 0f 10 96 b8 01 	movsd  0x1b8(%r14),%xmm2
   289de:	00 00 
   289e0:	f2 0f 58 c2          	addsd  %xmm2,%xmm0
   289e4:	f2 41 0f 10 9e 10 02 	movsd  0x210(%r14),%xmm3
   289eb:	00 00 
   289ed:	f2 0f 58 c3          	addsd  %xmm3,%xmm0
   289f1:	f2 41 0f 10 a6 68 02 	movsd  0x268(%r14),%xmm4
   289f8:	00 00 
   289fa:	f2 0f 58 c4          	addsd  %xmm4,%xmm0
   289fe:	f2 41 0f 10 ae c0 02 	movsd  0x2c0(%r14),%xmm5
   28a05:	00 00 
   28a07:	f2 0f 58 c5          	addsd  %xmm5,%xmm0
   28a0b:	f2 41 0f 10 b6 18 03 	movsd  0x318(%r14),%xmm6
   28a12:	00 00 
   28a14:	f2 0f 58 c6          	addsd  %xmm6,%xmm0
   28a18:	f2 44 0f 10 1d 1f 43 	movsd  -0x1bce1(%rip),%xmm11        # cd40 <GCC_except_table0+0xb4>
   28a1f:	fe ff 
   28a21:	f2 41 0f 5e c3       	divsd  %xmm11,%xmm0
   28a26:	f2 0f 59 05 4a 43 fe 	mulsd  -0x1bcb6(%rip),%xmm0        # cd78 <GCC_except_table0+0xec>
   28a2d:	ff 
   28a2e:	f2 0f 5f 05 22 43 fe 	maxsd  -0x1bcde(%rip),%xmm0        # cd58 <GCC_except_table0+0xcc>
   28a35:	ff 
   28a36:	f2 44 0f 58 d0       	addsd  %xmm0,%xmm10
   28a3b:	f2 45 0f 11 16       	movsd  %xmm10,(%r14)
   28a40:	f2 0f 58 f8          	addsd  %xmm0,%xmm7
   28a44:	f2 41 0f 11 7e 58    	movsd  %xmm7,0x58(%r14)
   28a4a:	f2 44 0f 58 c0       	addsd  %xmm0,%xmm8
   28a4f:	f2 45 0f 11 86 b0 00 	movsd  %xmm8,0xb0(%r14)
   28a56:	00 00 
   28a58:	f2 44 0f 58 c8       	addsd  %xmm0,%xmm9
   28a5d:	f2 45 0f 11 8e 08 01 	movsd  %xmm9,0x108(%r14)
   28a64:	00 00 
   28a66:	f2 41 0f 59 c3       	mulsd  %xmm11,%xmm0
   28a6b:	f2 0f 58 c8          	addsd  %xmm0,%xmm1
   28a6f:	f2 41 0f 11 8e 60 01 	movsd  %xmm1,0x160(%r14)
   28a76:	00 00 
   28a78:	f2 0f 58 d0          	addsd  %xmm0,%xmm2
   28a7c:	f2 41 0f 11 96 b8 01 	movsd  %xmm2,0x1b8(%r14)
   28a83:	00 00 
   28a85:	f2 0f 58 d8          	addsd  %xmm0,%xmm3
   28a89:	f2 41 0f 11 9e 10 02 	movsd  %xmm3,0x210(%r14)
   28a90:	00 00 
   28a92:	f2 0f 58 e0          	addsd  %xmm0,%xmm4
   28a96:	f2 41 0f 11 a6 68 02 	movsd  %xmm4,0x268(%r14)
   28a9d:	00 00 
   28a9f:	f2 0f 58 e8          	addsd  %xmm0,%xmm5
   28aa3:	f2 41 0f 11 ae c0 02 	movsd  %xmm5,0x2c0(%r14)
   28aaa:	00 00 
   28aac:	f2 0f 58 c6          	addsd  %xmm6,%xmm0
   28ab0:	f2 41 0f 11 86 18 03 	movsd  %xmm0,0x318(%r14)
   28ab7:	00 00 
   28ab9:	48 8b 5c 24 50       	mov    0x50(%rsp),%rbx
   28abe:	48 8d 04 1b          	lea    (%rbx,%rbx,1),%rax
   28ac2:	48 c1 e3 05          	shl    $0x5,%rbx
   28ac6:	48 29 c3             	sub    %rax,%rbx
   28ac9:	48 c7 44 24 48 00 00 	movq   $0x0,0x48(%rsp)
   28ad0:	00 00 
   28ad2:	48 89 9c 24 a8 00 00 	mov    %rbx,0xa8(%rsp)
   28ad9:	00 
   28ada:	48 c7 84 24 c8 00 00 	movq   $0x0,0xc8(%rsp)
   28ae1:	00 00 00 00 00 
   28ae6:	48 c7 84 24 d0 00 00 	movq   $0xa,0xd0(%rsp)
   28aed:	00 0a 00 00 00 
   28af2:	48 8d 44 24 78       	lea    0x78(%rsp),%rax
   28af7:	48 89 84 24 b0 00 00 	mov    %rax,0xb0(%rsp)
   28afe:	00 
   28aff:	48 8d 84 24 a8 00 00 	lea    0xa8(%rsp),%rax
   28b06:	00 
   28b07:	48 89 84 24 b8 00 00 	mov    %rax,0xb8(%rsp)
   28b0e:	00 
   28b0f:	48 8d 44 24 48       	lea    0x48(%rsp),%rax
   28b14:	48 89 84 24 c0 00 00 	mov    %rax,0xc0(%rsp)
   28b1b:	00 
   28b1c:	48 8d 7c 24 28       	lea    0x28(%rsp),%rdi
   28b21:	4c 8d bc 24 b0 00 00 	lea    0xb0(%rsp),%r15
   28b28:	00 
   28b29:	4c 89 fe             	mov    %r15,%rsi
   28b2c:	e8 5f 10 00 00       	call   29b90 <<alloc::vec::Vec<f64> as alloc::vec::spec_from_iter_nested::SpecFromIterNested<f64, core::iter::adapters::map::Map<core::ops::range::Range<usize>, xdremux_engine::apple_photographic_styles::apple_style_revert_local_data::{closure#3}>>>::from_iter>
   28b31:	48 8b 6c 24 30       	mov    0x30(%rsp),%rbp
   28b36:	4c 8b 44 24 38       	mov    0x38(%rsp),%r8
   28b3b:	ba 64 00 00 00       	mov    $0x64,%edx
   28b40:	4c 89 ff             	mov    %r15,%rdi
   28b43:	4c 89 f6             	mov    %r14,%rsi
   28b46:	48 89 e9             	mov    %rbp,%rcx
   28b49:	e8 42 dd ff ff       	call   26890 <xdremux_engine::apple_photographic_styles::solve_linear_system>
   28b4e:	80 bc 24 b0 00 00 00 	cmpb   $0xff,0xb0(%rsp)
   28b55:	ff 
   28b56:	74 38                	je     28b90 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1300>
   28b58:	ff 15 aa ab 04 00    	call   *0x4abaa(%rip)        # 73708 <_DYNAMIC+0x288>
   28b5e:	bf 50 00 00 00       	mov    $0x50,%edi
   28b63:	be 08 00 00 00       	mov    $0x8,%esi
   28b68:	ff 15 da ac 04 00    	call   *0x4acda(%rip)        # 73848 <_DYNAMIC+0x3c8>
   28b6e:	48 85 c0             	test   %rax,%rax
   28b71:	0f 84 19 0d 00 00    	je     29890 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x2000>
   28b77:	49 89 c5             	mov    %rax,%r13
   28b7a:	48 83 c0 50          	add    $0x50,%rax
   28b7e:	41 bf 0a 00 00 00    	mov    $0xa,%r15d
   28b84:	eb 32                	jmp    28bb8 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1328>
   28b86:	66 2e 0f 1f 84 00 00 	cs nopw 0x0(%rax,%rax,1)
   28b8d:	00 00 00 
   28b90:	4c 8b bc 24 b8 00 00 	mov    0xb8(%rsp),%r15
   28b97:	00 
   28b98:	4c 8b ac 24 c0 00 00 	mov    0xc0(%rsp),%r13
   28b9f:	00 
   28ba0:	48 8b 84 24 c8 00 00 	mov    0xc8(%rsp),%rax
   28ba7:	00 
   28ba8:	48 85 c0             	test   %rax,%rax
   28bab:	74 7f                	je     28c2c <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x139c>
   28bad:	48 8d 04 c5 00 00 00 	lea    0x0(,%rax,8),%rax
   28bb4:	00 
   28bb5:	4c 01 e8             	add    %r13,%rax
   28bb8:	4c 8d 1d 51 48 fe ff 	lea    -0x1b7af(%rip),%r11        # d410 <GCC_except_table0+0x784>
   28bbf:	66 0f 28 15 b9 47 fe 	movapd -0x1b847(%rip),%xmm2        # d380 <GCC_except_table0+0x6f4>
   28bc6:	ff 
   28bc7:	48 8b 8c 24 38 01 00 	mov    0x138(%rsp),%rcx
   28bce:	00 
   28bcf:	48 8b b4 24 40 01 00 	mov    0x140(%rsp),%rsi
   28bd6:	00 
   28bd7:	31 d2                	xor    %edx,%edx
   28bd9:	4d 89 e8             	mov    %r13,%r8
   28bdc:	0f 1f 40 00          	nopl   0x0(%rax)
   28be0:	4c 8b 4c 24 48       	mov    0x48(%rsp),%r9
   28be5:	49 8d 3c 14          	lea    (%r12,%rdx,1),%rdi
   28be9:	4c 01 cf             	add    %r9,%rdi
   28bec:	48 39 f7             	cmp    %rsi,%rdi
   28bef:	0f 83 21 0b 00 00    	jae    29716 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1e86>
   28bf5:	49 01 d1             	add    %rdx,%r9
   28bf8:	45 31 d2             	xor    %r10d,%r10d
   28bfb:	49 83 f9 0c          	cmp    $0xc,%r9
   28bff:	41 0f 93 c2          	setae  %r10b
   28c03:	f2 43 0f 10 04 d3    	movsd  (%r11,%r10,8),%xmm0
   28c09:	66 0f 28 c8          	movapd %xmm0,%xmm1
   28c0d:	66 0f 57 ca          	xorpd  %xmm2,%xmm1
   28c11:	f2 41 0f 5f 08       	maxsd  (%r8),%xmm1
   28c16:	49 83 c0 08          	add    $0x8,%r8
   28c1a:	f2 0f 5d c1          	minsd  %xmm1,%xmm0
   28c1e:	f2 0f 11 04 f9       	movsd  %xmm0,(%rcx,%rdi,8)
   28c23:	48 83 c2 03          	add    $0x3,%rdx
   28c27:	49 39 c0             	cmp    %rax,%r8
   28c2a:	75 b4                	jne    28be0 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1350>
   28c2c:	4d 85 ff             	test   %r15,%r15
   28c2f:	74 15                	je     28c46 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x13b6>
   28c31:	49 c1 e7 03          	shl    $0x3,%r15
   28c35:	ba 08 00 00 00       	mov    $0x8,%edx
   28c3a:	4c 89 ef             	mov    %r13,%rdi
   28c3d:	4c 89 fe             	mov    %r15,%rsi
   28c40:	ff 15 8a aa 04 00    	call   *0x4aa8a(%rip)        # 736d0 <_DYNAMIC+0x250>
   28c46:	48 8b 74 24 28       	mov    0x28(%rsp),%rsi
   28c4b:	48 85 f6             	test   %rsi,%rsi
   28c4e:	74 12                	je     28c62 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x13d2>
   28c50:	48 c1 e6 03          	shl    $0x3,%rsi
   28c54:	ba 08 00 00 00       	mov    $0x8,%edx
   28c59:	48 89 ef             	mov    %rbp,%rdi
   28c5c:	ff 15 6e aa 04 00    	call   *0x4aa6e(%rip)        # 736d0 <_DYNAMIC+0x250>
   28c62:	48 c7 44 24 48 01 00 	movq   $0x1,0x48(%rsp)
   28c69:	00 00 
   28c6b:	48 89 9c 24 a8 00 00 	mov    %rbx,0xa8(%rsp)
   28c72:	00 
   28c73:	48 c7 84 24 c8 00 00 	movq   $0x0,0xc8(%rsp)
   28c7a:	00 00 00 00 00 
   28c7f:	48 c7 84 24 d0 00 00 	movq   $0xa,0xd0(%rsp)
   28c86:	00 0a 00 00 00 
   28c8b:	48 8d 44 24 78       	lea    0x78(%rsp),%rax
   28c90:	48 89 84 24 b0 00 00 	mov    %rax,0xb0(%rsp)
   28c97:	00 
   28c98:	48 8d 84 24 a8 00 00 	lea    0xa8(%rsp),%rax
   28c9f:	00 
   28ca0:	48 89 84 24 b8 00 00 	mov    %rax,0xb8(%rsp)
   28ca7:	00 
   28ca8:	48 8d 44 24 48       	lea    0x48(%rsp),%rax
   28cad:	48 89 84 24 c0 00 00 	mov    %rax,0xc0(%rsp)
   28cb4:	00 
   28cb5:	48 8d 7c 24 28       	lea    0x28(%rsp),%rdi
   28cba:	4c 8d bc 24 b0 00 00 	lea    0xb0(%rsp),%r15
   28cc1:	00 
   28cc2:	4c 89 fe             	mov    %r15,%rsi
   28cc5:	e8 c6 0e 00 00       	call   29b90 <<alloc::vec::Vec<f64> as alloc::vec::spec_from_iter_nested::SpecFromIterNested<f64, core::iter::adapters::map::Map<core::ops::range::Range<usize>, xdremux_engine::apple_photographic_styles::apple_style_revert_local_data::{closure#3}>>>::from_iter>
   28cca:	48 8b 6c 24 30       	mov    0x30(%rsp),%rbp
   28ccf:	4c 8b 44 24 38       	mov    0x38(%rsp),%r8
   28cd4:	ba 64 00 00 00       	mov    $0x64,%edx
   28cd9:	4c 89 ff             	mov    %r15,%rdi
   28cdc:	4c 89 f6             	mov    %r14,%rsi
   28cdf:	48 89 e9             	mov    %rbp,%rcx
   28ce2:	e8 a9 db ff ff       	call   26890 <xdremux_engine::apple_photographic_styles::solve_linear_system>
   28ce7:	80 bc 24 b0 00 00 00 	cmpb   $0xff,0xb0(%rsp)
   28cee:	ff 
   28cef:	74 2f                	je     28d20 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1490>
   28cf1:	ff 15 11 aa 04 00    	call   *0x4aa11(%rip)        # 73708 <_DYNAMIC+0x288>
   28cf7:	bf 50 00 00 00       	mov    $0x50,%edi
   28cfc:	be 08 00 00 00       	mov    $0x8,%esi
   28d01:	ff 15 41 ab 04 00    	call   *0x4ab41(%rip)        # 73848 <_DYNAMIC+0x3c8>
   28d07:	48 85 c0             	test   %rax,%rax
   28d0a:	0f 84 80 0b 00 00    	je     29890 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x2000>
   28d10:	49 89 c5             	mov    %rax,%r13
   28d13:	48 83 c0 50          	add    $0x50,%rax
   28d17:	41 bf 0a 00 00 00    	mov    $0xa,%r15d
   28d1d:	eb 29                	jmp    28d48 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x14b8>
   28d1f:	90                   	nop
   28d20:	4c 8b bc 24 b8 00 00 	mov    0xb8(%rsp),%r15
   28d27:	00 
   28d28:	4c 8b ac 24 c0 00 00 	mov    0xc0(%rsp),%r13
   28d2f:	00 
   28d30:	48 8b 84 24 c8 00 00 	mov    0xc8(%rsp),%rax
   28d37:	00 
   28d38:	48 85 c0             	test   %rax,%rax
   28d3b:	74 7f                	je     28dbc <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x152c>
   28d3d:	48 8d 04 c5 00 00 00 	lea    0x0(,%rax,8),%rax
   28d44:	00 
   28d45:	4c 01 e8             	add    %r13,%rax
   28d48:	4c 8d 1d c1 46 fe ff 	lea    -0x1b93f(%rip),%r11        # d410 <GCC_except_table0+0x784>
   28d4f:	66 0f 28 15 29 46 fe 	movapd -0x1b9d7(%rip),%xmm2        # d380 <GCC_except_table0+0x6f4>
   28d56:	ff 
   28d57:	48 8b 8c 24 38 01 00 	mov    0x138(%rsp),%rcx
   28d5e:	00 
   28d5f:	48 8b b4 24 40 01 00 	mov    0x140(%rsp),%rsi
   28d66:	00 
   28d67:	31 d2                	xor    %edx,%edx
   28d69:	4d 89 e8             	mov    %r13,%r8
   28d6c:	0f 1f 40 00          	nopl   0x0(%rax)
   28d70:	4c 8b 4c 24 48       	mov    0x48(%rsp),%r9
   28d75:	49 8d 3c 14          	lea    (%r12,%rdx,1),%rdi
   28d79:	4c 01 cf             	add    %r9,%rdi
   28d7c:	48 39 f7             	cmp    %rsi,%rdi
   28d7f:	0f 83 91 09 00 00    	jae    29716 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1e86>
   28d85:	49 01 d1             	add    %rdx,%r9
   28d88:	45 31 d2             	xor    %r10d,%r10d
   28d8b:	49 83 f9 0c          	cmp    $0xc,%r9
   28d8f:	41 0f 93 c2          	setae  %r10b
   28d93:	f2 43 0f 10 04 d3    	movsd  (%r11,%r10,8),%xmm0
   28d99:	66 0f 28 c8          	movapd %xmm0,%xmm1
   28d9d:	66 0f 57 ca          	xorpd  %xmm2,%xmm1
   28da1:	f2 41 0f 5f 08       	maxsd  (%r8),%xmm1
   28da6:	49 83 c0 08          	add    $0x8,%r8
   28daa:	f2 0f 5d c1          	minsd  %xmm1,%xmm0
   28dae:	f2 0f 11 04 f9       	movsd  %xmm0,(%rcx,%rdi,8)
   28db3:	48 83 c2 03          	add    $0x3,%rdx
   28db7:	49 39 c0             	cmp    %rax,%r8
   28dba:	75 b4                	jne    28d70 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x14e0>
   28dbc:	4d 85 ff             	test   %r15,%r15
   28dbf:	74 15                	je     28dd6 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1546>
   28dc1:	49 c1 e7 03          	shl    $0x3,%r15
   28dc5:	ba 08 00 00 00       	mov    $0x8,%edx
   28dca:	4c 89 ef             	mov    %r13,%rdi
   28dcd:	4c 89 fe             	mov    %r15,%rsi
   28dd0:	ff 15 fa a8 04 00    	call   *0x4a8fa(%rip)        # 736d0 <_DYNAMIC+0x250>
   28dd6:	48 8b 74 24 28       	mov    0x28(%rsp),%rsi
   28ddb:	48 85 f6             	test   %rsi,%rsi
   28dde:	74 12                	je     28df2 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1562>
   28de0:	48 c1 e6 03          	shl    $0x3,%rsi
   28de4:	ba 08 00 00 00       	mov    $0x8,%edx
   28de9:	48 89 ef             	mov    %rbp,%rdi
   28dec:	ff 15 de a8 04 00    	call   *0x4a8de(%rip)        # 736d0 <_DYNAMIC+0x250>
   28df2:	48 c7 44 24 48 02 00 	movq   $0x2,0x48(%rsp)
   28df9:	00 00 
   28dfb:	48 89 9c 24 a8 00 00 	mov    %rbx,0xa8(%rsp)
   28e02:	00 
   28e03:	48 c7 84 24 c8 00 00 	movq   $0x0,0xc8(%rsp)
   28e0a:	00 00 00 00 00 
   28e0f:	48 c7 84 24 d0 00 00 	movq   $0xa,0xd0(%rsp)
   28e16:	00 0a 00 00 00 
   28e1b:	48 8d 44 24 78       	lea    0x78(%rsp),%rax
   28e20:	48 89 84 24 b0 00 00 	mov    %rax,0xb0(%rsp)
   28e27:	00 
   28e28:	48 8d 84 24 a8 00 00 	lea    0xa8(%rsp),%rax
   28e2f:	00 
   28e30:	48 89 84 24 b8 00 00 	mov    %rax,0xb8(%rsp)
   28e37:	00 
   28e38:	48 8d 44 24 48       	lea    0x48(%rsp),%rax
   28e3d:	48 89 84 24 c0 00 00 	mov    %rax,0xc0(%rsp)
   28e44:	00 
   28e45:	48 8d 7c 24 28       	lea    0x28(%rsp),%rdi
   28e4a:	4c 8d bc 24 b0 00 00 	lea    0xb0(%rsp),%r15
   28e51:	00 
   28e52:	4c 89 fe             	mov    %r15,%rsi
   28e55:	e8 36 0d 00 00       	call   29b90 <<alloc::vec::Vec<f64> as alloc::vec::spec_from_iter_nested::SpecFromIterNested<f64, core::iter::adapters::map::Map<core::ops::range::Range<usize>, xdremux_engine::apple_photographic_styles::apple_style_revert_local_data::{closure#3}>>>::from_iter>
   28e5a:	48 8b 6c 24 30       	mov    0x30(%rsp),%rbp
   28e5f:	4c 8b 44 24 38       	mov    0x38(%rsp),%r8
   28e64:	ba 64 00 00 00       	mov    $0x64,%edx
   28e69:	4c 89 ff             	mov    %r15,%rdi
   28e6c:	4c 89 f6             	mov    %r14,%rsi
   28e6f:	48 89 e9             	mov    %rbp,%rcx
   28e72:	e8 19 da ff ff       	call   26890 <xdremux_engine::apple_photographic_styles::solve_linear_system>
   28e77:	80 bc 24 b0 00 00 00 	cmpb   $0xff,0xb0(%rsp)
   28e7e:	ff 
   28e7f:	74 4f                	je     28ed0 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1640>
   28e81:	ff 15 81 a8 04 00    	call   *0x4a881(%rip)        # 73708 <_DYNAMIC+0x288>
   28e87:	bf 50 00 00 00       	mov    $0x50,%edi
   28e8c:	be 08 00 00 00       	mov    $0x8,%esi
   28e91:	ff 15 b1 a9 04 00    	call   *0x4a9b1(%rip)        # 73848 <_DYNAMIC+0x3c8>
   28e97:	48 85 c0             	test   %rax,%rax
   28e9a:	0f 84 f0 09 00 00    	je     29890 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x2000>
   28ea0:	49 89 c5             	mov    %rax,%r13
   28ea3:	48 83 c0 50          	add    $0x50,%rax
   28ea7:	41 bf 0a 00 00 00    	mov    $0xa,%r15d
   28ead:	48 8b 1d 1c a8 04 00 	mov    0x4a81c(%rip),%rbx        # 736d0 <_DYNAMIC+0x250>
   28eb4:	4c 8d 1d 55 45 fe ff 	lea    -0x1baab(%rip),%r11        # d410 <GCC_except_table0+0x784>
   28ebb:	66 0f 28 15 bd 44 fe 	movapd -0x1bb43(%rip),%xmm2        # d380 <GCC_except_table0+0x6f4>
   28ec2:	ff 
   28ec3:	eb 4d                	jmp    28f12 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1682>
   28ec5:	66 66 2e 0f 1f 84 00 	data16 cs nopw 0x0(%rax,%rax,1)
   28ecc:	00 00 00 00 
   28ed0:	4c 8b bc 24 b8 00 00 	mov    0xb8(%rsp),%r15
   28ed7:	00 
   28ed8:	4c 8b ac 24 c0 00 00 	mov    0xc0(%rsp),%r13
   28edf:	00 
   28ee0:	48 8b 84 24 c8 00 00 	mov    0xc8(%rsp),%rax
   28ee7:	00 
   28ee8:	48 85 c0             	test   %rax,%rax
   28eeb:	4c 8d 1d 1e 45 fe ff 	lea    -0x1bae2(%rip),%r11        # d410 <GCC_except_table0+0x784>
   28ef2:	66 0f 28 15 86 44 fe 	movapd -0x1bb7a(%rip),%xmm2        # d380 <GCC_except_table0+0x6f4>
   28ef9:	ff 
   28efa:	0f 84 b3 00 00 00    	je     28fb3 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1723>
   28f00:	48 8d 04 c5 00 00 00 	lea    0x0(,%rax,8),%rax
   28f07:	00 
   28f08:	4c 01 e8             	add    %r13,%rax
   28f0b:	48 8b 1d be a7 04 00 	mov    0x4a7be(%rip),%rbx        # 736d0 <_DYNAMIC+0x250>
   28f12:	48 8b 8c 24 38 01 00 	mov    0x138(%rsp),%rcx
   28f19:	00 
   28f1a:	48 8b b4 24 40 01 00 	mov    0x140(%rsp),%rsi
   28f21:	00 
   28f22:	31 d2                	xor    %edx,%edx
   28f24:	4d 89 e8             	mov    %r13,%r8
   28f27:	66 0f 1f 84 00 00 00 	nopw   0x0(%rax,%rax,1)
   28f2e:	00 00 
   28f30:	4c 8b 4c 24 48       	mov    0x48(%rsp),%r9
   28f35:	49 8d 3c 14          	lea    (%r12,%rdx,1),%rdi
   28f39:	4c 01 cf             	add    %r9,%rdi
   28f3c:	48 39 f7             	cmp    %rsi,%rdi
   28f3f:	0f 83 d1 07 00 00    	jae    29716 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1e86>
   28f45:	49 01 d1             	add    %rdx,%r9
   28f48:	45 31 d2             	xor    %r10d,%r10d
   28f4b:	49 83 f9 0c          	cmp    $0xc,%r9
   28f4f:	41 0f 93 c2          	setae  %r10b
   28f53:	f2 43 0f 10 04 d3    	movsd  (%r11,%r10,8),%xmm0
   28f59:	66 0f 28 c8          	movapd %xmm0,%xmm1
   28f5d:	66 0f 57 ca          	xorpd  %xmm2,%xmm1
   28f61:	f2 41 0f 5f 08       	maxsd  (%r8),%xmm1
   28f66:	49 83 c0 08          	add    $0x8,%r8
   28f6a:	f2 0f 5d c1          	minsd  %xmm1,%xmm0
   28f6e:	f2 0f 11 04 f9       	movsd  %xmm0,(%rcx,%rdi,8)
   28f73:	48 83 c2 03          	add    $0x3,%rdx
   28f77:	49 39 c0             	cmp    %rax,%r8
   28f7a:	75 b4                	jne    28f30 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x16a0>
   28f7c:	4d 85 ff             	test   %r15,%r15
   28f7f:	74 11                	je     28f92 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1702>
   28f81:	49 c1 e7 03          	shl    $0x3,%r15
   28f85:	ba 08 00 00 00       	mov    $0x8,%edx
   28f8a:	4c 89 ef             	mov    %r13,%rdi
   28f8d:	4c 89 fe             	mov    %r15,%rsi
   28f90:	ff d3                	call   *%rbx
   28f92:	48 8b 74 24 28       	mov    0x28(%rsp),%rsi
   28f97:	48 85 f6             	test   %rsi,%rsi
   28f9a:	0f 84 f0 f7 ff ff    	je     28790 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0xf00>
   28fa0:	48 c1 e6 03          	shl    $0x3,%rsi
   28fa4:	ba 08 00 00 00       	mov    $0x8,%edx
   28fa9:	48 89 ef             	mov    %rbp,%rdi
   28fac:	ff d3                	call   *%rbx
   28fae:	e9 dd f7 ff ff       	jmp    28790 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0xf00>
   28fb3:	48 8b 1d 16 a7 04 00 	mov    0x4a716(%rip),%rbx        # 736d0 <_DYNAMIC+0x250>
   28fba:	4d 85 ff             	test   %r15,%r15
   28fbd:	75 c2                	jne    28f81 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x16f1>
   28fbf:	eb d1                	jmp    28f92 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1702>
   28fc1:	48 8b 84 24 e0 01 00 	mov    0x1e0(%rsp),%rax
   28fc8:	00 
   28fc9:	48 05 e0 e3 ff ff    	add    $0xffffffffffffe3e0,%rax
   28fcf:	48 8b b4 24 20 01 00 	mov    0x120(%rsp),%rsi
   28fd6:	00 
   28fd7:	48 81 c6 00 e1 00 00 	add    $0xe100,%rsi
   28fde:	48 8b 94 24 d0 01 00 	mov    0x1d0(%rsp),%rdx
   28fe5:	00 
   28fe6:	48 81 c2 20 1c 00 00 	add    $0x1c20,%rdx
   28fed:	48 8b 8c 24 78 01 00 	mov    0x178(%rsp),%rcx
   28ff4:	00 
   28ff5:	48 81 c1 70 08 00 00 	add    $0x870,%rcx
   28ffc:	48 8b bc 24 18 01 00 	mov    0x118(%rsp),%rdi
   29003:	00 
   29004:	48 83 ff 0c          	cmp    $0xc,%rdi
   29008:	0f 85 a7 f6 ff ff    	jne    286b5 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0xe25>
   2900e:	ff 15 f4 a6 04 00    	call   *0x4a6f4(%rip)        # 73708 <_DYNAMIC+0x288>
   29014:	41 bf 80 ca 00 00    	mov    $0xca80,%r15d
   2901a:	41 bd 01 00 00 00    	mov    $0x1,%r13d
   29020:	bf 80 ca 00 00       	mov    $0xca80,%edi
   29025:	be 01 00 00 00       	mov    $0x1,%esi
   2902a:	ff 15 e0 a6 04 00    	call   *0x4a6e0(%rip)        # 73710 <_DYNAMIC+0x290>
   29030:	48 85 c0             	test   %rax,%rax
   29033:	0f 84 69 08 00 00    	je     298a2 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x2012>
   29039:	48 89 c5             	mov    %rax,%rbp
   2903c:	48 c7 44 24 28 80 ca 	movq   $0xca80,0x28(%rsp)
   29043:	00 00 
   29045:	48 89 44 24 30       	mov    %rax,0x30(%rsp)
   2904a:	48 c7 44 24 38 00 00 	movq   $0x0,0x38(%rsp)
   29051:	00 00 
   29053:	48 8b 84 24 38 01 00 	mov    0x138(%rsp),%rax
   2905a:	00 
   2905b:	48 8b 8c 24 40 01 00 	mov    0x140(%rsp),%rcx
   29062:	00 
   29063:	48 89 4c 24 20       	mov    %rcx,0x20(%rsp)
   29068:	48 89 84 24 50 01 00 	mov    %rax,0x150(%rsp)
   2906f:	00 
   29070:	4c 8d 60 10          	lea    0x10(%rax),%r12
   29074:	31 db                	xor    %ebx,%ebx
   29076:	45 31 ed             	xor    %r13d,%r13d
   29079:	31 d2                	xor    %edx,%edx
   2907b:	e9 9e 00 00 00       	jmp    2911e <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x188e>
   29080:	c6 07 02             	movb   $0x2,(%rdi)
   29083:	48 89 f8             	mov    %rdi,%rax
   29086:	48 81 c4 78 02 00 00 	add    $0x278,%rsp
   2908d:	5b                   	pop    %rbx
   2908e:	41 5c                	pop    %r12
   29090:	41 5d                	pop    %r13
   29092:	41 5e                	pop    %r14
   29094:	41 5f                	pop    %r15
   29096:	5d                   	pop    %rbp
   29097:	c3                   	ret
   29098:	49 8d 76 64          	lea    0x64(%r14),%rsi
   2909c:	48 8d 0d 05 81 04 00 	lea    0x48105(%rip),%rcx        # 711a8 <anon.e11926f7acc299ef6543ba90fd675eca.18.llvm.1662880743899104407+0x258>
   290a3:	ba 80 51 01 00       	mov    $0x15180,%edx
   290a8:	4c 89 f7             	mov    %r14,%rdi
   290ab:	eb 07                	jmp    290b4 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1824>
   290ad:	48 8d 0d dc 80 04 00 	lea    0x480dc(%rip),%rcx        # 71190 <anon.e11926f7acc299ef6543ba90fd675eca.18.llvm.1662880743899104407+0x240>
   290b4:	ff 15 5e a6 04 00    	call   *0x4a65e(%rip)        # 73718 <_DYNAMIC+0x298>
   290ba:	e9 11 08 00 00       	jmp    298d0 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x2040>
   290bf:	48 01 ef             	add    %rbp,%rdi
   290c2:	48 8d 15 4f 80 04 00 	lea    0x4804f(%rip),%rdx        # 71118 <anon.e11926f7acc299ef6543ba90fd675eca.18.llvm.1662880743899104407+0x1c8>
   290c9:	be 80 51 01 00       	mov    $0x15180,%esi
   290ce:	eb 12                	jmp    290e2 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1852>
   290d0:	49 01 ef             	add    %rbp,%r15
   290d3:	48 8d 15 56 80 04 00 	lea    0x48056(%rip),%rdx        # 71130 <anon.e11926f7acc299ef6543ba90fd675eca.18.llvm.1662880743899104407+0x1e0>
   290da:	be 64 00 00 00       	mov    $0x64,%esi
   290df:	4c 89 ff             	mov    %r15,%rdi
   290e2:	4c 8b 34 24          	mov    (%rsp),%r14
   290e6:	ff 15 bc a6 04 00    	call   *0x4a6bc(%rip)        # 737a8 <_DYNAMIC+0x328>
   290ec:	e9 df 07 00 00       	jmp    298d0 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x2040>
   290f1:	4c 8b 64 24 70       	mov    0x70(%rsp),%r12
   290f6:	49 81 c4 80 43 00 00 	add    $0x4380,%r12
   290fd:	48 8b 9c 24 90 01 00 	mov    0x190(%rsp),%rbx
   29104:	00 
   29105:	48 81 c3 70 08 00 00 	add    $0x870,%rbx
   2910c:	48 8b 94 24 80 01 00 	mov    0x180(%rsp),%rdx
   29113:	00 
   29114:	48 83 fa 0c          	cmp    $0xc,%rdx
   29118:	0f 84 21 06 00 00    	je     2973f <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1eaf>
   2911e:	48 ff c2             	inc    %rdx
   29121:	48 89 94 24 80 01 00 	mov    %rdx,0x180(%rsp)
   29128:	00 
   29129:	48 89 9c 24 90 01 00 	mov    %rbx,0x190(%rsp)
   29130:	00 
   29131:	4c 89 64 24 70       	mov    %r12,0x70(%rsp)
   29136:	31 c0                	xor    %eax,%eax
   29138:	eb 29                	jmp    29163 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x18d3>
   2913a:	4c 8b a4 24 90 00 00 	mov    0x90(%rsp),%r12
   29141:	00 
   29142:	49 81 c4 80 07 00 00 	add    $0x780,%r12
   29149:	48 8b 5c 24 50       	mov    0x50(%rsp),%rbx
   2914e:	48 81 c3 f0 00 00 00 	add    $0xf0,%rbx
   29155:	48 8b 84 24 00 01 00 	mov    0x100(%rsp),%rax
   2915c:	00 
   2915d:	48 83 f8 09          	cmp    $0x9,%rax
   29161:	74 8e                	je     290f1 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1861>
   29163:	48 ff c0             	inc    %rax
   29166:	48 89 84 24 00 01 00 	mov    %rax,0x100(%rsp)
   2916d:	00 
   2916e:	48 89 5c 24 50       	mov    %rbx,0x50(%rsp)
   29173:	4c 89 a4 24 90 00 00 	mov    %r12,0x90(%rsp)
   2917a:	00 
   2917b:	31 c0                	xor    %eax,%eax
   2917d:	eb 17                	jmp    29196 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1906>
   2917f:	90                   	nop
   29180:	49 81 c4 f0 00 00 00 	add    $0xf0,%r12
   29187:	48 83 c3 1e          	add    $0x1e,%rbx
   2918b:	48 8b 44 24 60       	mov    0x60(%rsp),%rax
   29190:	48 83 f8 08          	cmp    $0x8,%rax
   29194:	74 a4                	je     2913a <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x18aa>
   29196:	48 ff c0             	inc    %rax
   29199:	48 89 44 24 60       	mov    %rax,0x60(%rsp)
   2919e:	49 c7 c7 fd ff ff ff 	mov    $0xfffffffffffffffd,%r15
   291a5:	66 66 2e 0f 1f 84 00 	data16 cs nopw 0x0(%rax,%rax,1)
   291ac:	00 00 00 00 
   291b0:	66 0f 57 c0          	xorpd  %xmm0,%xmm0
   291b4:	f3 0f 11 04 24       	movss  %xmm0,(%rsp)
   291b9:	49 83 ff 08          	cmp    $0x8,%r15
   291bd:	77 12                	ja     291d1 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1941>
   291bf:	48 8d 05 ce 6f fe ff 	lea    -0x19032(%rip),%rax        # 10194 <anon.e11926f7acc299ef6543ba90fd675eca.16.llvm.1662880743899104407+0x87e>
   291c6:	f3 42 0f 10 04 b8    	movss  (%rax,%r15,4),%xmm0
   291cc:	f3 0f 11 04 24       	movss  %xmm0,(%rsp)
   291d1:	4a 8d 3c 3b          	lea    (%rbx,%r15,1),%rdi
   291d5:	48 83 c7 03          	add    $0x3,%rdi
   291d9:	48 3b 7c 24 20       	cmp    0x20(%rsp),%rdi
   291de:	0f 83 44 05 00 00    	jae    29728 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1e98>
   291e4:	f2 43 0f 10 44 fc 08 	movsd  0x8(%r12,%r15,8),%xmm0
   291eb:	f2 0f 5a c0          	cvtsd2ss %xmm0,%xmm0
   291ef:	f3 0f 10 0c 24       	movss  (%rsp),%xmm1
   291f4:	f3 0f 58 c8          	addss  %xmm0,%xmm1
   291f8:	f3 0f 11 0c 24       	movss  %xmm1,(%rsp)
   291fd:	48 8b 05 5c a8 04 00 	mov    0x4a85c(%rip),%rax        # 73a60 <_DYNAMIC+0x5e0>
   29204:	48 8b 00             	mov    (%rax),%rax
   29207:	48 85 c0             	test   %rax,%rax
   2920a:	0f 84 ec 00 00 00    	je     292fc <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1a6c>
   29210:	48 b9 00 00 00 00 00 	movabs $0x40000000000000,%rcx
   29217:	00 40 00 
   2921a:	48 85 c8             	test   %rcx,%rax
   2921d:	0f 85 ed 00 00 00    	jne    29310 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1a80>
   29223:	66 0f 6e 04 24       	movd   (%rsp),%xmm0
   29228:	66 0f 7e c1          	movd   %xmm0,%ecx
   2922c:	89 c8                	mov    %ecx,%eax
   2922e:	25 00 00 00 80       	and    $0x80000000,%eax
   29233:	89 ce                	mov    %ecx,%esi
   29235:	81 e6 00 00 80 7f    	and    $0x7f800000,%esi
   2923b:	89 ca                	mov    %ecx,%edx
   2923d:	81 e2 ff ff 7f 00    	and    $0x7fffff,%edx
   29243:	81 fe 00 00 80 7f    	cmp    $0x7f800000,%esi
   29249:	75 25                	jne    29270 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x19e0>
   2924b:	31 c9                	xor    %ecx,%ecx
   2924d:	85 d2                	test   %edx,%edx
   2924f:	0f 95 c1             	setne  %cl
   29252:	c1 e1 09             	shl    $0x9,%ecx
   29255:	c1 e8 10             	shr    $0x10,%eax
   29258:	c1 ea 0d             	shr    $0xd,%edx
   2925b:	09 c2                	or     %eax,%edx
   2925d:	09 ca                	or     %ecx,%edx
   2925f:	81 ca 00 7c 00 00    	or     $0x7c00,%edx
   29265:	41 89 d6             	mov    %edx,%r14d
   29268:	e9 b0 00 00 00       	jmp    2931d <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1a8d>
   2926d:	0f 1f 00             	nopl   (%rax)
   29270:	c1 e8 10             	shr    $0x10,%eax
   29273:	81 fe 00 00 00 47    	cmp    $0x47000000,%esi
   29279:	76 0d                	jbe    29288 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x19f8>
   2927b:	0d 00 7c 00 00       	or     $0x7c00,%eax
   29280:	41 89 c6             	mov    %eax,%r14d
   29283:	e9 95 00 00 00       	jmp    2931d <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1a8d>
   29288:	81 fe 00 00 80 38    	cmp    $0x38800000,%esi
   2928e:	73 42                	jae    292d2 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1a42>
   29290:	81 fe 00 00 00 33    	cmp    $0x33000000,%esi
   29296:	72 e8                	jb     29280 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x19f0>
   29298:	c1 ee 17             	shr    $0x17,%esi
   2929b:	81 ca 00 00 80 00    	or     $0x800000,%edx
   292a1:	b1 7e                	mov    $0x7e,%cl
   292a3:	40 28 f1             	sub    %sil,%cl
   292a6:	41 89 d6             	mov    %edx,%r14d
   292a9:	41 d3 ee             	shr    %cl,%r14d
   292ac:	b9 1d 00 00 00       	mov    $0x1d,%ecx
   292b1:	29 f1                	sub    %esi,%ecx
   292b3:	83 e1 1f             	and    $0x1f,%ecx
   292b6:	0f a3 ca             	bt     %ecx,%edx
   292b9:	73 12                	jae    292cd <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1a3d>
   292bb:	be 03 00 00 00       	mov    $0x3,%esi
   292c0:	d3 e6                	shl    %cl,%esi
   292c2:	ff ce                	dec    %esi
   292c4:	21 d6                	and    %edx,%esi
   292c6:	83 fe 01             	cmp    $0x1,%esi
   292c9:	41 83 de ff          	sbb    $0xffffffff,%r14d
   292cd:	41 09 c6             	or     %eax,%r14d
   292d0:	eb 4b                	jmp    2931d <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1a8d>
   292d2:	c1 ee 0d             	shr    $0xd,%esi
   292d5:	c1 ea 0d             	shr    $0xd,%edx
   292d8:	89 cf                	mov    %ecx,%edi
   292da:	c1 ef 0c             	shr    $0xc,%edi
   292dd:	f7 c1 ff 2f 00 00    	test   $0x2fff,%ecx
   292e3:	0f 95 c1             	setne  %cl
   292e6:	40 20 f9             	and    %dil,%cl
   292e9:	01 f2                	add    %esi,%edx
   292eb:	81 c2 00 40 00 00    	add    $0x4000,%edx
   292f1:	09 c2                	or     %eax,%edx
   292f3:	44 0f b6 f1          	movzbl %cl,%r14d
   292f7:	41 01 d6             	add    %edx,%r14d
   292fa:	eb 21                	jmp    2931d <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1a8d>
   292fc:	ff 15 66 a7 04 00    	call   *0x4a766(%rip)        # 73a68 <_DYNAMIC+0x5e8>
   29302:	48 0f ba e0 36       	bt     $0x36,%rax
   29307:	0f 83 16 ff ff ff    	jae    29223 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1993>
   2930d:	0f 1f 00             	nopl   (%rax)
   29310:	66 0f 6e 04 24       	movd   (%rsp),%xmm0
   29315:	e8 d6 06 00 00       	call   299f0 <half::binary16::arch::x86::f32_to_f16_x86_f16c>
   2931a:	41 89 c6             	mov    %eax,%r14d
   2931d:	48 8b 44 24 28       	mov    0x28(%rsp),%rax
   29322:	4c 29 e8             	sub    %r13,%rax
   29325:	48 83 f8 01          	cmp    $0x1,%rax
   29329:	0f 86 68 01 00 00    	jbe    29497 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1c07>
   2932f:	66 46 89 74 2d 00    	mov    %r14w,0x0(%rbp,%r13,1)
   29335:	49 83 c5 02          	add    $0x2,%r13
   29339:	4c 89 6c 24 38       	mov    %r13,0x38(%rsp)
   2933e:	49 8d 47 01          	lea    0x1(%r15),%rax
   29342:	66 0f ef c0          	pxor   %xmm0,%xmm0
   29346:	66 0f 7e 04 24       	movd   %xmm0,(%rsp)
   2934b:	48 83 f8 08          	cmp    $0x8,%rax
   2934f:	77 13                	ja     29364 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1ad4>
   29351:	48 8d 05 3c 6e fe ff 	lea    -0x191c4(%rip),%rax        # 10194 <anon.e11926f7acc299ef6543ba90fd675eca.16.llvm.1662880743899104407+0x87e>
   29358:	66 42 0f 6e 44 b8 04 	movd   0x4(%rax,%r15,4),%xmm0
   2935f:	66 0f 7e 04 24       	movd   %xmm0,(%rsp)
   29364:	4a 8d 3c 3b          	lea    (%rbx,%r15,1),%rdi
   29368:	48 83 c7 04          	add    $0x4,%rdi
   2936c:	48 3b 7c 24 20       	cmp    0x20(%rsp),%rdi
   29371:	0f 83 b1 03 00 00    	jae    29728 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1e98>
   29377:	f2 43 0f 10 44 fc 10 	movsd  0x10(%r12,%r15,8),%xmm0
   2937e:	f2 0f 5a c0          	cvtsd2ss %xmm0,%xmm0
   29382:	f3 0f 10 0c 24       	movss  (%rsp),%xmm1
   29387:	f3 0f 58 c8          	addss  %xmm0,%xmm1
   2938b:	f3 0f 11 0c 24       	movss  %xmm1,(%rsp)
   29390:	48 8b 05 c9 a6 04 00 	mov    0x4a6c9(%rip),%rax        # 73a60 <_DYNAMIC+0x5e0>
   29397:	48 8b 00             	mov    (%rax),%rax
   2939a:	48 85 c0             	test   %rax,%rax
   2939d:	0f 84 20 01 00 00    	je     294c3 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1c33>
   293a3:	48 b9 00 00 00 00 00 	movabs $0x40000000000000,%rcx
   293aa:	00 40 00 
   293ad:	48 85 c8             	test   %rcx,%rax
   293b0:	0f 85 2a 01 00 00    	jne    294e0 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1c50>
   293b6:	66 0f 6e 04 24       	movd   (%rsp),%xmm0
   293bb:	66 0f 7e c1          	movd   %xmm0,%ecx
   293bf:	89 c8                	mov    %ecx,%eax
   293c1:	25 00 00 00 80       	and    $0x80000000,%eax
   293c6:	89 ce                	mov    %ecx,%esi
   293c8:	81 e6 00 00 80 7f    	and    $0x7f800000,%esi
   293ce:	89 ca                	mov    %ecx,%edx
   293d0:	81 e2 ff ff 7f 00    	and    $0x7fffff,%edx
   293d6:	81 fe 00 00 80 7f    	cmp    $0x7f800000,%esi
   293dc:	75 22                	jne    29400 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1b70>
   293de:	31 c9                	xor    %ecx,%ecx
   293e0:	85 d2                	test   %edx,%edx
   293e2:	0f 95 c1             	setne  %cl
   293e5:	c1 e1 09             	shl    $0x9,%ecx
   293e8:	c1 e8 10             	shr    $0x10,%eax
   293eb:	c1 ea 0d             	shr    $0xd,%edx
   293ee:	09 c2                	or     %eax,%edx
   293f0:	09 ca                	or     %ecx,%edx
   293f2:	81 ca 00 7c 00 00    	or     $0x7c00,%edx
   293f8:	41 89 d6             	mov    %edx,%r14d
   293fb:	e9 ed 00 00 00       	jmp    294ed <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1c5d>
   29400:	c1 e8 10             	shr    $0x10,%eax
   29403:	81 fe 00 00 00 47    	cmp    $0x47000000,%esi
   29409:	76 0d                	jbe    29418 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1b88>
   2940b:	0d 00 7c 00 00       	or     $0x7c00,%eax
   29410:	41 89 c6             	mov    %eax,%r14d
   29413:	e9 d5 00 00 00       	jmp    294ed <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1c5d>
   29418:	81 fe 00 00 80 38    	cmp    $0x38800000,%esi
   2941e:	73 10                	jae    29430 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1ba0>
   29420:	81 fe 00 00 00 33    	cmp    $0x33000000,%esi
   29426:	73 35                	jae    2945d <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1bcd>
   29428:	41 89 c6             	mov    %eax,%r14d
   2942b:	e9 bd 00 00 00       	jmp    294ed <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1c5d>
   29430:	c1 ee 0d             	shr    $0xd,%esi
   29433:	c1 ea 0d             	shr    $0xd,%edx
   29436:	89 cf                	mov    %ecx,%edi
   29438:	c1 ef 0c             	shr    $0xc,%edi
   2943b:	f7 c1 ff 2f 00 00    	test   $0x2fff,%ecx
   29441:	0f 95 c1             	setne  %cl
   29444:	40 20 f9             	and    %dil,%cl
   29447:	01 f2                	add    %esi,%edx
   29449:	81 c2 00 40 00 00    	add    $0x4000,%edx
   2944f:	09 c2                	or     %eax,%edx
   29451:	44 0f b6 f1          	movzbl %cl,%r14d
   29455:	41 01 d6             	add    %edx,%r14d
   29458:	e9 90 00 00 00       	jmp    294ed <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1c5d>
   2945d:	c1 ee 17             	shr    $0x17,%esi
   29460:	81 ca 00 00 80 00    	or     $0x800000,%edx
   29466:	b1 7e                	mov    $0x7e,%cl
   29468:	40 28 f1             	sub    %sil,%cl
   2946b:	41 89 d6             	mov    %edx,%r14d
   2946e:	41 d3 ee             	shr    %cl,%r14d
   29471:	b9 1d 00 00 00       	mov    $0x1d,%ecx
   29476:	29 f1                	sub    %esi,%ecx
   29478:	83 e1 1f             	and    $0x1f,%ecx
   2947b:	0f a3 ca             	bt     %ecx,%edx
   2947e:	73 12                	jae    29492 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1c02>
   29480:	be 03 00 00 00       	mov    $0x3,%esi
   29485:	d3 e6                	shl    %cl,%esi
   29487:	ff ce                	dec    %esi
   29489:	21 d6                	and    %edx,%esi
   2948b:	83 fe 01             	cmp    $0x1,%esi
   2948e:	41 83 de ff          	sbb    $0xffffffff,%r14d
   29492:	41 09 c6             	or     %eax,%r14d
   29495:	eb 56                	jmp    294ed <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1c5d>
   29497:	ba 02 00 00 00       	mov    $0x2,%edx
   2949c:	b9 01 00 00 00       	mov    $0x1,%ecx
   294a1:	41 b8 01 00 00 00    	mov    $0x1,%r8d
   294a7:	48 8d 7c 24 28       	lea    0x28(%rsp),%rdi
   294ac:	4c 89 ee             	mov    %r13,%rsi
   294af:	e8 2c 08 00 00       	call   29ce0 <<alloc::raw_vec::RawVecInner<_>>::reserve::do_reserve_and_handle::<alloc::alloc::Global>>
   294b4:	48 8b 6c 24 30       	mov    0x30(%rsp),%rbp
   294b9:	4c 8b 6c 24 38       	mov    0x38(%rsp),%r13
   294be:	e9 6c fe ff ff       	jmp    2932f <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1a9f>
   294c3:	ff 15 9f a5 04 00    	call   *0x4a59f(%rip)        # 73a68 <_DYNAMIC+0x5e8>
   294c9:	48 0f ba e0 36       	bt     $0x36,%rax
   294ce:	0f 83 e2 fe ff ff    	jae    293b6 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1b26>
   294d4:	66 66 66 2e 0f 1f 84 	data16 data16 cs nopw 0x0(%rax,%rax,1)
   294db:	00 00 00 00 00 
   294e0:	66 0f 6e 04 24       	movd   (%rsp),%xmm0
   294e5:	e8 06 05 00 00       	call   299f0 <half::binary16::arch::x86::f32_to_f16_x86_f16c>
   294ea:	41 89 c6             	mov    %eax,%r14d
   294ed:	48 8b 44 24 28       	mov    0x28(%rsp),%rax
   294f2:	4c 29 e8             	sub    %r13,%rax
   294f5:	48 83 f8 01          	cmp    $0x1,%rax
   294f9:	0f 86 78 01 00 00    	jbe    29677 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1de7>
   294ff:	48 8b 6c 24 30       	mov    0x30(%rsp),%rbp
   29504:	66 46 89 74 2d 00    	mov    %r14w,0x0(%rbp,%r13,1)
   2950a:	49 83 c5 02          	add    $0x2,%r13
   2950e:	4c 89 6c 24 38       	mov    %r13,0x38(%rsp)
   29513:	49 8d 47 02          	lea    0x2(%r15),%rax
   29517:	66 0f ef c0          	pxor   %xmm0,%xmm0
   2951b:	66 0f 7e 04 24       	movd   %xmm0,(%rsp)
   29520:	48 83 f8 08          	cmp    $0x8,%rax
   29524:	77 13                	ja     29539 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1ca9>
   29526:	48 8d 05 67 6c fe ff 	lea    -0x19399(%rip),%rax        # 10194 <anon.e11926f7acc299ef6543ba90fd675eca.16.llvm.1662880743899104407+0x87e>
   2952d:	66 42 0f 6e 44 b8 08 	movd   0x8(%rax,%r15,4),%xmm0
   29534:	66 0f 7e 04 24       	movd   %xmm0,(%rsp)
   29539:	4a 8d 3c 3b          	lea    (%rbx,%r15,1),%rdi
   2953d:	48 83 c7 05          	add    $0x5,%rdi
   29541:	48 3b 7c 24 20       	cmp    0x20(%rsp),%rdi
   29546:	0f 83 dc 01 00 00    	jae    29728 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1e98>
   2954c:	f2 43 0f 10 44 fc 18 	movsd  0x18(%r12,%r15,8),%xmm0
   29553:	f2 0f 5a c0          	cvtsd2ss %xmm0,%xmm0
   29557:	f3 0f 10 0c 24       	movss  (%rsp),%xmm1
   2955c:	f3 0f 58 c8          	addss  %xmm0,%xmm1
   29560:	f3 0f 11 0c 24       	movss  %xmm1,(%rsp)
   29565:	48 8b 05 f4 a4 04 00 	mov    0x4a4f4(%rip),%rax        # 73a60 <_DYNAMIC+0x5e0>
   2956c:	48 8b 00             	mov    (%rax),%rax
   2956f:	48 85 c0             	test   %rax,%rax
   29572:	0f 84 26 01 00 00    	je     2969e <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1e0e>
   29578:	48 b9 00 00 00 00 00 	movabs $0x40000000000000,%rcx
   2957f:	00 40 00 
   29582:	48 85 c8             	test   %rcx,%rax
   29585:	0f 85 25 01 00 00    	jne    296b0 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1e20>
   2958b:	66 0f 6e 04 24       	movd   (%rsp),%xmm0
   29590:	66 0f 7e c1          	movd   %xmm0,%ecx
   29594:	89 c8                	mov    %ecx,%eax
   29596:	25 00 00 00 80       	and    $0x80000000,%eax
   2959b:	89 ce                	mov    %ecx,%esi
   2959d:	81 e6 00 00 80 7f    	and    $0x7f800000,%esi
   295a3:	89 ca                	mov    %ecx,%edx
   295a5:	81 e2 ff ff 7f 00    	and    $0x7fffff,%edx
   295ab:	81 fe 00 00 80 7f    	cmp    $0x7f800000,%esi
   295b1:	75 2d                	jne    295e0 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1d50>
   295b3:	31 c9                	xor    %ecx,%ecx
   295b5:	85 d2                	test   %edx,%edx
   295b7:	0f 95 c1             	setne  %cl
   295ba:	c1 e1 09             	shl    $0x9,%ecx
   295bd:	c1 e8 10             	shr    $0x10,%eax
   295c0:	c1 ea 0d             	shr    $0xd,%edx
   295c3:	09 c2                	or     %eax,%edx
   295c5:	09 ca                	or     %ecx,%edx
   295c7:	81 ca 00 7c 00 00    	or     $0x7c00,%edx
   295cd:	41 89 d6             	mov    %edx,%r14d
   295d0:	e9 e8 00 00 00       	jmp    296bd <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1e2d>
   295d5:	66 66 2e 0f 1f 84 00 	data16 cs nopw 0x0(%rax,%rax,1)
   295dc:	00 00 00 00 
   295e0:	c1 e8 10             	shr    $0x10,%eax
   295e3:	81 fe 00 00 00 47    	cmp    $0x47000000,%esi
   295e9:	76 0d                	jbe    295f8 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1d68>
   295eb:	0d 00 7c 00 00       	or     $0x7c00,%eax
   295f0:	41 89 c6             	mov    %eax,%r14d
   295f3:	e9 c5 00 00 00       	jmp    296bd <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1e2d>
   295f8:	81 fe 00 00 80 38    	cmp    $0x38800000,%esi
   295fe:	73 10                	jae    29610 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1d80>
   29600:	81 fe 00 00 00 33    	cmp    $0x33000000,%esi
   29606:	73 35                	jae    2963d <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1dad>
   29608:	41 89 c6             	mov    %eax,%r14d
   2960b:	e9 ad 00 00 00       	jmp    296bd <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1e2d>
   29610:	c1 ee 0d             	shr    $0xd,%esi
   29613:	c1 ea 0d             	shr    $0xd,%edx
   29616:	89 cf                	mov    %ecx,%edi
   29618:	c1 ef 0c             	shr    $0xc,%edi
   2961b:	f7 c1 ff 2f 00 00    	test   $0x2fff,%ecx
   29621:	0f 95 c1             	setne  %cl
   29624:	40 20 f9             	and    %dil,%cl
   29627:	01 f2                	add    %esi,%edx
   29629:	81 c2 00 40 00 00    	add    $0x4000,%edx
   2962f:	09 c2                	or     %eax,%edx
   29631:	44 0f b6 f1          	movzbl %cl,%r14d
   29635:	41 01 d6             	add    %edx,%r14d
   29638:	e9 80 00 00 00       	jmp    296bd <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1e2d>
   2963d:	c1 ee 17             	shr    $0x17,%esi
   29640:	81 ca 00 00 80 00    	or     $0x800000,%edx
   29646:	b1 7e                	mov    $0x7e,%cl
   29648:	40 28 f1             	sub    %sil,%cl
   2964b:	41 89 d6             	mov    %edx,%r14d
   2964e:	41 d3 ee             	shr    %cl,%r14d
   29651:	b9 1d 00 00 00       	mov    $0x1d,%ecx
   29656:	29 f1                	sub    %esi,%ecx
   29658:	83 e1 1f             	and    $0x1f,%ecx
   2965b:	0f a3 ca             	bt     %ecx,%edx
   2965e:	73 12                	jae    29672 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1de2>
   29660:	be 03 00 00 00       	mov    $0x3,%esi
   29665:	d3 e6                	shl    %cl,%esi
   29667:	ff ce                	dec    %esi
   29669:	21 d6                	and    %edx,%esi
   2966b:	83 fe 01             	cmp    $0x1,%esi
   2966e:	41 83 de ff          	sbb    $0xffffffff,%r14d
   29672:	41 09 c6             	or     %eax,%r14d
   29675:	eb 46                	jmp    296bd <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1e2d>
   29677:	ba 02 00 00 00       	mov    $0x2,%edx
   2967c:	b9 01 00 00 00       	mov    $0x1,%ecx
   29681:	41 b8 01 00 00 00    	mov    $0x1,%r8d
   29687:	48 8d 7c 24 28       	lea    0x28(%rsp),%rdi
   2968c:	4c 89 ee             	mov    %r13,%rsi
   2968f:	e8 4c 06 00 00       	call   29ce0 <<alloc::raw_vec::RawVecInner<_>>::reserve::do_reserve_and_handle::<alloc::alloc::Global>>
   29694:	4c 8b 6c 24 38       	mov    0x38(%rsp),%r13
   29699:	e9 61 fe ff ff       	jmp    294ff <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1c6f>
   2969e:	ff 15 c4 a3 04 00    	call   *0x4a3c4(%rip)        # 73a68 <_DYNAMIC+0x5e8>
   296a4:	48 0f ba e0 36       	bt     $0x36,%rax
   296a9:	0f 83 dc fe ff ff    	jae    2958b <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1cfb>
   296af:	90                   	nop
   296b0:	66 0f 6e 04 24       	movd   (%rsp),%xmm0
   296b5:	e8 36 03 00 00       	call   299f0 <half::binary16::arch::x86::f32_to_f16_x86_f16c>
   296ba:	41 89 c6             	mov    %eax,%r14d
   296bd:	48 8b 44 24 28       	mov    0x28(%rsp),%rax
   296c2:	4c 29 e8             	sub    %r13,%rax
   296c5:	48 83 f8 01          	cmp    $0x1,%rax
   296c9:	76 22                	jbe    296ed <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1e5d>
   296cb:	66 46 89 74 2d 00    	mov    %r14w,0x0(%rbp,%r13,1)
   296d1:	49 83 c5 02          	add    $0x2,%r13
   296d5:	4c 89 6c 24 38       	mov    %r13,0x38(%rsp)
   296da:	49 83 c7 03          	add    $0x3,%r15
   296de:	49 83 ff 1b          	cmp    $0x1b,%r15
   296e2:	0f 85 c8 fa ff ff    	jne    291b0 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1920>
   296e8:	e9 93 fa ff ff       	jmp    29180 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x18f0>
   296ed:	ba 02 00 00 00       	mov    $0x2,%edx
   296f2:	b9 01 00 00 00       	mov    $0x1,%ecx
   296f7:	41 b8 01 00 00 00    	mov    $0x1,%r8d
   296fd:	48 8d 7c 24 28       	lea    0x28(%rsp),%rdi
   29702:	4c 89 ee             	mov    %r13,%rsi
   29705:	e8 d6 05 00 00       	call   29ce0 <<alloc::raw_vec::RawVecInner<_>>::reserve::do_reserve_and_handle::<alloc::alloc::Global>>
   2970a:	48 8b 6c 24 30       	mov    0x30(%rsp),%rbp
   2970f:	4c 8b 6c 24 38       	mov    0x38(%rsp),%r13
   29714:	eb b5                	jmp    296cb <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1e3b>
   29716:	48 8d 15 e3 79 04 00 	lea    0x479e3(%rip),%rdx        # 71100 <anon.e11926f7acc299ef6543ba90fd675eca.18.llvm.1662880743899104407+0x1b0>
   2971d:	ff 15 85 a0 04 00    	call   *0x4a085(%rip)        # 737a8 <_DYNAMIC+0x328>
   29723:	e9 a8 01 00 00       	jmp    298d0 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x2040>
   29728:	48 8d 15 b9 79 04 00 	lea    0x479b9(%rip),%rdx        # 710e8 <anon.e11926f7acc299ef6543ba90fd675eca.18.llvm.1662880743899104407+0x198>
   2972f:	48 8b 74 24 20       	mov    0x20(%rsp),%rsi
   29734:	ff 15 6e a0 04 00    	call   *0x4a06e(%rip)        # 737a8 <_DYNAMIC+0x328>
   2973a:	e9 91 01 00 00       	jmp    298d0 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x2040>
   2973f:	48 8d bc 24 b0 00 00 	lea    0xb0(%rsp),%rdi
   29746:	00 
   29747:	48 89 ee             	mov    %rbp,%rsi
   2974a:	4c 89 ea             	mov    %r13,%rdx
   2974d:	ff 15 35 a3 04 00    	call   *0x4a335(%rip)        # 73a88 <_DYNAMIC+0x608>
   29753:	83 bc 24 b0 00 00 00 	cmpl   $0x1,0xb0(%rsp)
   2975a:	01 
   2975b:	75 79                	jne    297d6 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1f46>
   2975d:	48 8b 84 24 b8 00 00 	mov    0xb8(%rsp),%rax
   29764:	00 
   29765:	f2 0f 10 84 24 c0 00 	movsd  0xc0(%rsp),%xmm0
   2976c:	00 00 
   2976e:	f3 0f 10 8c 24 d0 00 	movss  0xd0(%rsp),%xmm1
   29775:	00 00 
   29777:	0f b6 8c 24 d4 00 00 	movzbl 0xd4(%rsp),%ecx
   2977e:	00 
   2977f:	0f b6 94 24 d7 00 00 	movzbl 0xd7(%rsp),%edx
   29786:	00 
   29787:	48 8b b4 24 10 01 00 	mov    0x110(%rsp),%rsi
   2978e:	00 
   2978f:	88 56 1f             	mov    %dl,0x1f(%rsi)
   29792:	0f b7 94 24 d5 00 00 	movzwl 0xd5(%rsp),%edx
   29799:	00 
   2979a:	66 89 56 1d          	mov    %dx,0x1d(%rsi)
   2979e:	f2 0f 10 94 24 c8 00 	movsd  0xc8(%rsp),%xmm2
   297a5:	00 00 
   297a7:	48 89 06             	mov    %rax,(%rsi)
   297aa:	f2 0f 11 46 08       	movsd  %xmm0,0x8(%rsi)
   297af:	f2 0f 11 56 10       	movsd  %xmm2,0x10(%rsi)
   297b4:	f3 0f 11 4e 18       	movss  %xmm1,0x18(%rsi)
   297b9:	88 4e 1c             	mov    %cl,0x1c(%rsi)
   297bc:	48 8b 74 24 28       	mov    0x28(%rsp),%rsi
   297c1:	48 85 f6             	test   %rsi,%rsi
   297c4:	74 2d                	je     297f3 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1f63>
   297c6:	ba 01 00 00 00       	mov    $0x1,%edx
   297cb:	48 89 ef             	mov    %rbp,%rdi
   297ce:	ff 15 fc 9e 04 00    	call   *0x49efc(%rip)        # 736d0 <_DYNAMIC+0x250>
   297d4:	eb 1d                	jmp    297f3 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1f63>
   297d6:	48 8b 44 24 38       	mov    0x38(%rsp),%rax
   297db:	48 8b 8c 24 10 01 00 	mov    0x110(%rsp),%rcx
   297e2:	00 
   297e3:	48 89 41 18          	mov    %rax,0x18(%rcx)
   297e7:	0f 10 44 24 28       	movups 0x28(%rsp),%xmm0
   297ec:	0f 11 41 08          	movups %xmm0,0x8(%rcx)
   297f0:	c6 01 ff             	movb   $0xff,(%rcx)
   297f3:	48 8b b4 24 30 01 00 	mov    0x130(%rsp),%rsi
   297fa:	00 
   297fb:	48 85 f6             	test   %rsi,%rsi
   297fe:	74 17                	je     29817 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1f87>
   29800:	48 c1 e6 03          	shl    $0x3,%rsi
   29804:	ba 08 00 00 00       	mov    $0x8,%edx
   29809:	48 8b bc 24 50 01 00 	mov    0x150(%rsp),%rdi
   29810:	00 
   29811:	ff 15 b9 9e 04 00    	call   *0x49eb9(%rip)        # 736d0 <_DYNAMIC+0x250>
   29817:	48 8b 74 24 78       	mov    0x78(%rsp),%rsi
   2981c:	48 85 f6             	test   %rsi,%rsi
   2981f:	74 17                	je     29838 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1fa8>
   29821:	48 8b bc 24 80 00 00 	mov    0x80(%rsp),%rdi
   29828:	00 
   29829:	48 c1 e6 03          	shl    $0x3,%rsi
   2982d:	ba 08 00 00 00       	mov    $0x8,%edx
   29832:	ff 15 98 9e 04 00    	call   *0x49e98(%rip)        # 736d0 <_DYNAMIC+0x250>
   29838:	be 00 8c 0a 00       	mov    $0xa8c00,%esi
   2983d:	ba 08 00 00 00       	mov    $0x8,%edx
   29842:	48 8b 7c 24 18       	mov    0x18(%rsp),%rdi
   29847:	ff 15 83 9e 04 00    	call   *0x49e83(%rip)        # 736d0 <_DYNAMIC+0x250>
   2984d:	48 8b bc 24 10 01 00 	mov    0x110(%rsp),%rdi
   29854:	00 
   29855:	e9 29 f8 ff ff       	jmp    29083 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x17f3>
   2985a:	48 89 c7             	mov    %rax,%rdi
   2985d:	48 8d 05 fc 78 04 00 	lea    0x478fc(%rip),%rax        # 71160 <anon.e11926f7acc299ef6543ba90fd675eca.18.llvm.1662880743899104407+0x210>
   29864:	eb 0a                	jmp    29870 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1fe0>
   29866:	48 89 c7             	mov    %rax,%rdi
   29869:	48 8d 05 08 79 04 00 	lea    0x47908(%rip),%rax        # 71178 <anon.e11926f7acc299ef6543ba90fd675eca.18.llvm.1662880743899104407+0x228>
   29870:	48 89 84 24 68 01 00 	mov    %rax,0x168(%rsp)
   29877:	00 
   29878:	48 8b b4 24 a8 01 00 	mov    0x1a8(%rsp),%rsi
   2987f:	00 
   29880:	48 8b 94 24 68 01 00 	mov    0x168(%rsp),%rdx
   29887:	00 
   29888:	ff 15 1a 9f 04 00    	call   *0x49f1a(%rip)        # 737a8 <_DYNAMIC+0x328>
   2988e:	eb 40                	jmp    298d0 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x2040>
   29890:	bf 08 00 00 00       	mov    $0x8,%edi
   29895:	be 50 00 00 00       	mov    $0x50,%esi
   2989a:	ff 15 60 9e 04 00    	call   *0x49e60(%rip)        # 73700 <_DYNAMIC+0x280>
   298a0:	eb 2e                	jmp    298d0 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x2040>
   298a2:	4c 89 ef             	mov    %r13,%rdi
   298a5:	4c 89 fe             	mov    %r15,%rsi
   298a8:	ff 15 52 9e 04 00    	call   *0x49e52(%rip)        # 73700 <_DYNAMIC+0x280>
   298ae:	eb 20                	jmp    298d0 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x2040>
   298b0:	bf 08 00 00 00       	mov    $0x8,%edi
   298b5:	be 00 8c 0a 00       	mov    $0xa8c00,%esi
   298ba:	ff 15 40 9e 04 00    	call   *0x49e40(%rip)        # 73700 <_DYNAMIC+0x280>
   298c0:	bf 08 00 00 00       	mov    $0x8,%edi
   298c5:	be 00 2a 03 00       	mov    $0x32a00,%esi
   298ca:	ff 15 30 9e 04 00    	call   *0x49e30(%rip)        # 73700 <_DYNAMIC+0x280>
   298d0:	0f 0b                	ud2
   298d2:	eb 2a                	jmp    298fe <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x206e>
   298d4:	49 89 c4             	mov    %rax,%r12
   298d7:	e9 d8 00 00 00       	jmp    299b4 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x2124>
   298dc:	e9 af 00 00 00       	jmp    29990 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x2100>
   298e1:	49 89 c4             	mov    %rax,%r12
   298e4:	e9 81 00 00 00       	jmp    2996a <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x20da>
   298e9:	e9 a2 00 00 00       	jmp    29990 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x2100>
   298ee:	49 89 c4             	mov    %rax,%r12
   298f1:	4c 8b 34 24          	mov    (%rsp),%r14
   298f5:	eb 3d                	jmp    29934 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x20a4>
   298f7:	eb 5b                	jmp    29954 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x20c4>
   298f9:	49 89 c4             	mov    %rax,%r12
   298fc:	eb 36                	jmp    29934 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x20a4>
   298fe:	49 89 c4             	mov    %rax,%r12
   29901:	48 8b 74 24 28       	mov    0x28(%rsp),%rsi
   29906:	48 85 f6             	test   %rsi,%rsi
   29909:	74 5f                	je     2996a <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x20da>
   2990b:	48 8b 7c 24 30       	mov    0x30(%rsp),%rdi
   29910:	ba 01 00 00 00       	mov    $0x1,%edx
   29915:	eb 4d                	jmp    29964 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x20d4>
   29917:	49 89 c4             	mov    %rax,%r12
   2991a:	4d 85 ff             	test   %r15,%r15
   2991d:	74 15                	je     29934 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x20a4>
   2991f:	49 c1 e7 03          	shl    $0x3,%r15
   29923:	ba 08 00 00 00       	mov    $0x8,%edx
   29928:	4c 89 ef             	mov    %r13,%rdi
   2992b:	4c 89 fe             	mov    %r15,%rsi
   2992e:	ff 15 9c 9d 04 00    	call   *0x49d9c(%rip)        # 736d0 <_DYNAMIC+0x250>
   29934:	48 8b 74 24 28       	mov    0x28(%rsp),%rsi
   29939:	48 85 f6             	test   %rsi,%rsi
   2993c:	74 19                	je     29957 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x20c7>
   2993e:	48 8b 7c 24 30       	mov    0x30(%rsp),%rdi
   29943:	48 c1 e6 03          	shl    $0x3,%rsi
   29947:	ba 08 00 00 00       	mov    $0x8,%edx
   2994c:	ff 15 7e 9d 04 00    	call   *0x49d7e(%rip)        # 736d0 <_DYNAMIC+0x250>
   29952:	eb 03                	jmp    29957 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x20c7>
   29954:	49 89 c4             	mov    %rax,%r12
   29957:	be 20 03 00 00       	mov    $0x320,%esi
   2995c:	ba 08 00 00 00       	mov    $0x8,%edx
   29961:	4c 89 f7             	mov    %r14,%rdi
   29964:	ff 15 66 9d 04 00    	call   *0x49d66(%rip)        # 736d0 <_DYNAMIC+0x250>
   2996a:	48 8b b4 24 30 01 00 	mov    0x130(%rsp),%rsi
   29971:	00 
   29972:	48 85 f6             	test   %rsi,%rsi
   29975:	74 1c                	je     29993 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x2103>
   29977:	48 8b bc 24 38 01 00 	mov    0x138(%rsp),%rdi
   2997e:	00 
   2997f:	48 c1 e6 03          	shl    $0x3,%rsi
   29983:	ba 08 00 00 00       	mov    $0x8,%edx
   29988:	ff 15 42 9d 04 00    	call   *0x49d42(%rip)        # 736d0 <_DYNAMIC+0x250>
   2998e:	eb 03                	jmp    29993 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x2103>
   29990:	49 89 c4             	mov    %rax,%r12
   29993:	48 8b 74 24 78       	mov    0x78(%rsp),%rsi
   29998:	48 85 f6             	test   %rsi,%rsi
   2999b:	74 17                	je     299b4 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x2124>
   2999d:	48 8b bc 24 80 00 00 	mov    0x80(%rsp),%rdi
   299a4:	00 
   299a5:	48 c1 e6 03          	shl    $0x3,%rsi
   299a9:	ba 08 00 00 00       	mov    $0x8,%edx
   299ae:	ff 15 1c 9d 04 00    	call   *0x49d1c(%rip)        # 736d0 <_DYNAMIC+0x250>
   299b4:	be 00 8c 0a 00       	mov    $0xa8c00,%esi
   299b9:	ba 08 00 00 00       	mov    $0x8,%edx
   299be:	48 8b 7c 24 18       	mov    0x18(%rsp),%rdi
   299c3:	ff 15 07 9d 04 00    	call   *0x49d07(%rip)        # 736d0 <_DYNAMIC+0x250>
   299c9:	4c 89 e7             	mov    %r12,%rdi
   299cc:	e8 5f 5d 04 00       	call   6f730 <_Unwind_Resume@plt>
