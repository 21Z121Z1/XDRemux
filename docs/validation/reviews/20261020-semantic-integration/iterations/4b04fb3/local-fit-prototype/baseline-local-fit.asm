
/tmp/xdremux-style-fit-accumulation-target/baseline/release/xdremux-style-fit-probe:     file format elf64-x86-64


Disassembly of section .text:

00000000000279d0 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data>:
   279d0:	55                   	push   %rbp
   279d1:	41 57                	push   %r15
   279d3:	41 56                	push   %r14
   279d5:	41 55                	push   %r13
   279d7:	41 54                	push   %r12
   279d9:	53                   	push   %rbx
   279da:	48 81 ec 68 02 00 00 	sub    $0x268,%rsp
   279e1:	4c 8b 76 20          	mov    0x20(%rsi),%r14
   279e5:	48 8b 4e 28          	mov    0x28(%rsi),%rcx
   279e9:	4c 89 f0             	mov    %r14,%rax
   279ec:	48 f7 e1             	mul    %rcx
   279ef:	0f 80 3c 01 00 00    	jo     27b31 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x161>
   279f5:	48 89 8c 24 40 01 00 	mov    %rcx,0x140(%rsp)
   279fc:	00 
   279fd:	b9 03 00 00 00       	mov    $0x3,%ecx
   27a02:	48 f7 e1             	mul    %rcx
   27a05:	0f 80 26 01 00 00    	jo     27b31 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x161>
   27a0b:	4d 85 f6             	test   %r14,%r14
   27a0e:	0f 84 1d 01 00 00    	je     27b31 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x161>
   27a14:	48 83 bc 24 40 01 00 	cmpq   $0x0,0x140(%rsp)
   27a1b:	00 00 
   27a1d:	0f 84 0e 01 00 00    	je     27b31 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x161>
   27a23:	48 89 c3             	mov    %rax,%rbx
   27a26:	48 39 46 08          	cmp    %rax,0x8(%rsi)
   27a2a:	0f 85 01 01 00 00    	jne    27b31 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x161>
   27a30:	48 39 5e 18          	cmp    %rbx,0x18(%rsi)
   27a34:	0f 85 f7 00 00 00    	jne    27b31 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x161>
   27a3a:	4c 8b 3e             	mov    (%rsi),%r15
   27a3d:	48 8d 04 9d 00 00 00 	lea    0x0(,%rbx,4),%rax
   27a44:	00 
   27a45:	48 85 db             	test   %rbx,%rbx
   27a48:	74 21                	je     27a6b <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x9b>
   27a4a:	31 c9                	xor    %ecx,%ecx
   27a4c:	0f 1f 40 00          	nopl   0x0(%rax)
   27a50:	41 8b 14 0f          	mov    (%r15,%rcx,1),%edx
   27a54:	01 d2                	add    %edx,%edx
   27a56:	81 fa ff ff ff fe    	cmp    $0xfeffffff,%edx
   27a5c:	0f 87 cf 00 00 00    	ja     27b31 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x161>
   27a62:	48 83 c1 04          	add    $0x4,%rcx
   27a66:	48 39 c8             	cmp    %rcx,%rax
   27a69:	75 e5                	jne    27a50 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x80>
   27a6b:	4c 8b 66 10          	mov    0x10(%rsi),%r12
   27a6f:	48 85 db             	test   %rbx,%rbx
   27a72:	74 27                	je     27a9b <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0xcb>
   27a74:	31 c9                	xor    %ecx,%ecx
   27a76:	66 2e 0f 1f 84 00 00 	cs nopw 0x0(%rax,%rax,1)
   27a7d:	00 00 00 
   27a80:	41 8b 14 0c          	mov    (%r12,%rcx,1),%edx
   27a84:	01 d2                	add    %edx,%edx
   27a86:	81 fa ff ff ff fe    	cmp    $0xfeffffff,%edx
   27a8c:	0f 87 9f 00 00 00    	ja     27b31 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x161>
   27a92:	48 83 c1 04          	add    $0x4,%rcx
   27a96:	48 39 c8             	cmp    %rcx,%rax
   27a99:	75 e5                	jne    27a80 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0xb0>
   27a9b:	48 89 bc 24 20 01 00 	mov    %rdi,0x120(%rsp)
   27aa2:	00 
   27aa3:	ff 15 cf bc 04 00    	call   *0x4bccf(%rip)        # 73778 <_DYNAMIC+0x288>
   27aa9:	bf 00 8c 0a 00       	mov    $0xa8c00,%edi
   27aae:	be 08 00 00 00       	mov    $0x8,%esi
   27ab3:	ff 15 ff bd 04 00    	call   *0x4bdff(%rip)        # 738b8 <_DYNAMIC+0x3c8>
   27ab9:	48 89 44 24 78       	mov    %rax,0x78(%rsp)
   27abe:	48 85 c0             	test   %rax,%rax
   27ac1:	0f 84 d3 1e 00 00    	je     2999a <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1fca>
   27ac7:	ff 15 ab bc 04 00    	call   *0x4bcab(%rip)        # 73778 <_DYNAMIC+0x288>
   27acd:	bf 00 2a 03 00       	mov    $0x32a00,%edi
   27ad2:	be 08 00 00 00       	mov    $0x8,%esi
   27ad7:	ff 15 db bd 04 00    	call   *0x4bddb(%rip)        # 738b8 <_DYNAMIC+0x3c8>
   27add:	48 85 c0             	test   %rax,%rax
   27ae0:	0f 84 c4 1e 00 00    	je     299aa <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1fda>
   27ae6:	48 c7 44 24 10 40 65 	movq   $0x6540,0x10(%rsp)
   27aed:	00 00 
   27aef:	48 89 44 24 18       	mov    %rax,0x18(%rsp)
   27af4:	48 c7 44 24 20 40 65 	movq   $0x6540,0x20(%rsp)
   27afb:	00 00 
   27afd:	31 ed                	xor    %ebp,%ebp
   27aff:	4c 8d ac 24 f0 00 00 	lea    0xf0(%rsp),%r13
   27b06:	00 
   27b07:	48 8b 94 24 40 01 00 	mov    0x140(%rsp),%rdx
   27b0e:	00 
   27b0f:	48 89 9c 24 90 00 00 	mov    %rbx,0x90(%rsp)
   27b16:	00 
   27b17:	4c 89 b4 24 18 02 00 	mov    %r14,0x218(%rsp)
   27b1e:	00 
   27b1f:	4c 89 bc 24 10 02 00 	mov    %r15,0x210(%rsp)
   27b26:	00 
   27b27:	4c 89 a4 24 08 02 00 	mov    %r12,0x208(%rsp)
   27b2e:	00 
   27b2f:	eb 3e                	jmp    27b6f <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x19f>
   27b31:	c6 07 02             	movb   $0x2,(%rdi)
   27b34:	48 89 f8             	mov    %rdi,%rax
   27b37:	48 81 c4 68 02 00 00 	add    $0x268,%rsp
   27b3e:	5b                   	pop    %rbx
   27b3f:	41 5c                	pop    %r12
   27b41:	41 5d                	pop    %r13
   27b43:	41 5e                	pop    %r14
   27b45:	41 5f                	pop    %r15
   27b47:	5d                   	pop    %rbp
   27b48:	c3                   	ret
   27b49:	0f 1f 80 00 00 00 00 	nopl   0x0(%rax)
   27b50:	49 89 d7             	mov    %rdx,%r15
   27b53:	48 8b 84 24 00 02 00 	mov    0x200(%rsp),%rax
   27b5a:	00 
   27b5b:	48 89 c5             	mov    %rax,%rbp
   27b5e:	48 8b 94 24 40 01 00 	mov    0x140(%rsp),%rdx
   27b65:	00 
   27b66:	48 39 d0             	cmp    %rdx,%rax
   27b69:	0f 84 00 0c 00 00    	je     2876f <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0xd9f>
   27b6f:	b9 09 00 00 00       	mov    $0x9,%ecx
   27b74:	4c 89 ef             	mov    %r13,%rdi
   27b77:	48 89 ee             	mov    %rbp,%rsi
   27b7a:	e8 01 ec ff ff       	call   26780 <xdremux_engine::apple_photographic_styles::spotlight_weights>
   27b7f:	48 8d 45 01          	lea    0x1(%rbp),%rax
   27b83:	48 89 84 24 00 02 00 	mov    %rax,0x200(%rsp)
   27b8a:	00 
   27b8b:	49 0f af ee          	imul   %r14,%rbp
   27b8f:	48 8b 84 24 f0 00 00 	mov    0xf0(%rsp),%rax
   27b96:	00 
   27b97:	f2 0f 10 84 24 f8 00 	movsd  0xf8(%rsp),%xmm0
   27b9e:	00 00 
   27ba0:	f2 0f 11 84 24 28 01 	movsd  %xmm0,0x128(%rsp)
   27ba7:	00 00 
   27ba9:	48 8b 8c 24 00 01 00 	mov    0x100(%rsp),%rcx
   27bb0:	00 
   27bb1:	f2 0f 10 84 24 08 01 	movsd  0x108(%rsp),%xmm0
   27bb8:	00 00 
   27bba:	f2 0f 11 84 24 c8 01 	movsd  %xmm0,0x1c8(%rsp)
   27bc1:	00 00 
   27bc3:	48 69 d0 20 03 00 00 	imul   $0x320,%rax,%rdx
   27bca:	48 89 94 24 b8 01 00 	mov    %rdx,0x1b8(%rsp)
   27bd1:	00 
   27bd2:	48 89 84 24 c0 01 00 	mov    %rax,0x1c0(%rsp)
   27bd9:	00 
   27bda:	48 69 d0 00 19 00 00 	imul   $0x1900,%rax,%rdx
   27be1:	48 8b 44 24 78       	mov    0x78(%rsp),%rax
   27be6:	48 01 c2             	add    %rax,%rdx
   27be9:	48 89 94 24 b0 01 00 	mov    %rdx,0x1b0(%rsp)
   27bf0:	00 
   27bf1:	48 69 d1 20 03 00 00 	imul   $0x320,%rcx,%rdx
   27bf8:	48 89 94 24 a0 01 00 	mov    %rdx,0x1a0(%rsp)
   27bff:	00 
   27c00:	48 89 8c 24 a8 01 00 	mov    %rcx,0x1a8(%rsp)
   27c07:	00 
   27c08:	48 69 c9 00 19 00 00 	imul   $0x1900,%rcx,%rcx
   27c0f:	48 01 c1             	add    %rax,%rcx
   27c12:	48 89 8c 24 98 01 00 	mov    %rcx,0x198(%rsp)
   27c19:	00 
   27c1a:	31 c9                	xor    %ecx,%ecx
   27c1c:	4c 89 fa             	mov    %r15,%rdx
   27c1f:	48 89 6c 24 08       	mov    %rbp,0x8(%rsp)
   27c24:	eb 3b                	jmp    27c61 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x291>
   27c26:	66 2e 0f 1f 84 00 00 	cs nopw 0x0(%rax,%rax,1)
   27c2d:	00 00 00 
   27c30:	48 8b 9c 24 90 00 00 	mov    0x90(%rsp),%rbx
   27c37:	00 
   27c38:	4c 8b b4 24 18 02 00 	mov    0x218(%rsp),%r14
   27c3f:	00 
   27c40:	48 8b 94 24 10 02 00 	mov    0x210(%rsp),%rdx
   27c47:	00 
   27c48:	4c 8b a4 24 08 02 00 	mov    0x208(%rsp),%r12
   27c4f:	00 
   27c50:	48 8b 4c 24 70       	mov    0x70(%rsp),%rcx
   27c55:	48 ff c1             	inc    %rcx
   27c58:	4c 39 f1             	cmp    %r14,%rcx
   27c5b:	0f 84 ef fe ff ff    	je     27b50 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x180>
   27c61:	48 8d 04 29          	lea    (%rcx,%rbp,1),%rax
   27c65:	48 8d 3c 40          	lea    (%rax,%rax,2),%rdi
   27c69:	48 39 df             	cmp    %rbx,%rdi
   27c6c:	0f 83 fc 1c 00 00    	jae    2996e <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1f9e>
   27c72:	48 8d 47 01          	lea    0x1(%rdi),%rax
   27c76:	48 39 d8             	cmp    %rbx,%rax
   27c79:	0f 83 c9 1c 00 00    	jae    29948 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1f78>
   27c7f:	48 89 4c 24 70       	mov    %rcx,0x70(%rsp)
   27c84:	48 8d 47 02          	lea    0x2(%rdi),%rax
   27c88:	48 39 d8             	cmp    %rbx,%rax
   27c8b:	0f 83 c6 1c 00 00    	jae    29957 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1f87>
   27c91:	4d 89 f7             	mov    %r14,%r15
   27c94:	f3 0f 10 04 ba       	movss  (%rdx,%rdi,4),%xmm0
   27c99:	0f 57 e4             	xorps  %xmm4,%xmm4
   27c9c:	f3 0f 5a e0          	cvtss2sd %xmm0,%xmm4
   27ca0:	f3 41 0f 10 04 bc    	movss  (%r12,%rdi,4),%xmm0
   27ca6:	f3 0f 11 84 24 58 01 	movss  %xmm0,0x158(%rsp)
   27cad:	00 00 
   27caf:	f3 41 0f 10 44 bc 04 	movss  0x4(%r12,%rdi,4),%xmm0
   27cb6:	f3 0f 11 84 24 50 01 	movss  %xmm0,0x150(%rsp)
   27cbd:	00 00 
   27cbf:	f3 41 0f 10 44 bc 08 	movss  0x8(%r12,%rdi,4),%xmm0
   27cc6:	f3 0f 11 84 24 48 01 	movss  %xmm0,0x148(%rsp)
   27ccd:	00 00 
   27ccf:	0f 28 c4             	movaps %xmm4,%xmm0
   27cd2:	48 b8 00 00 00 00 00 	movabs $0x3ff0000000000000,%rax
   27cd9:	00 f0 3f 
   27cdc:	48 89 84 24 a0 00 00 	mov    %rax,0xa0(%rsp)
   27ce3:	00 
   27ce4:	f2 0f 11 a4 24 a8 00 	movsd  %xmm4,0xa8(%rsp)
   27ceb:	00 00 
   27ced:	0f 5a 54 ba 04       	cvtps2pd 0x4(%rdx,%rdi,4),%xmm2
   27cf2:	f2 0f 59 c4          	mulsd  %xmm4,%xmm0
   27cf6:	66 0f 28 ea          	movapd %xmm2,%xmm5
   27cfa:	66 0f 15 ea          	unpckhpd %xmm2,%xmm5
   27cfe:	66 0f 28 cd          	movapd %xmm5,%xmm1
   27d02:	f2 0f 59 cd          	mulsd  %xmm5,%xmm1
   27d06:	66 0f 11 94 24 b0 00 	movupd %xmm2,0xb0(%rsp)
   27d0d:	00 00 
   27d0f:	f2 0f 11 84 24 c0 00 	movsd  %xmm0,0xc0(%rsp)
   27d16:	00 00 
   27d18:	66 0f 28 c4          	movapd %xmm4,%xmm0
   27d1c:	66 0f 14 c4          	unpcklpd %xmm4,%xmm0
   27d20:	66 0f 59 c2          	mulpd  %xmm2,%xmm0
   27d24:	66 0f 11 84 24 c8 00 	movupd %xmm0,0xc8(%rsp)
   27d2b:	00 00 
   27d2d:	66 0f 28 c2          	movapd %xmm2,%xmm0
   27d31:	66 0f 14 c2          	unpcklpd %xmm2,%xmm0
   27d35:	66 0f 59 c2          	mulpd  %xmm2,%xmm0
   27d39:	66 0f 11 84 24 d8 00 	movupd %xmm0,0xd8(%rsp)
   27d40:	00 00 
   27d42:	f2 0f 11 8c 24 e8 00 	movsd  %xmm1,0xe8(%rsp)
   27d49:	00 00 
   27d4b:	66 0f 28 c2          	movapd %xmm2,%xmm0
   27d4f:	f2 0f 58 c4          	addsd  %xmm4,%xmm0
   27d53:	f2 0f 58 c5          	addsd  %xmm5,%xmm0
   27d57:	f2 0f 5e 05 09 50 fe 	divsd  -0x1aff7(%rip),%xmm0        # cd68 <GCC_except_table0+0xe0>
   27d5e:	ff 
   27d5f:	66 0f 28 cc          	movapd %xmm4,%xmm1
   27d63:	f2 0f 5f ca          	maxsd  %xmm2,%xmm1
   27d67:	66 0f 29 94 24 d0 01 	movapd %xmm2,0x1d0(%rsp)
   27d6e:	00 00 
   27d70:	f2 0f c2 d2 03       	cmpunordsd %xmm2,%xmm2
   27d75:	66 0f 28 da          	movapd %xmm2,%xmm3
   27d79:	66 0f 55 d9          	andnpd %xmm1,%xmm3
   27d7d:	0f 29 a4 24 f0 01 00 	movaps %xmm4,0x1f0(%rsp)
   27d84:	00 
   27d85:	66 0f 54 d4          	andpd  %xmm4,%xmm2
   27d89:	66 0f 56 d3          	orpd   %xmm3,%xmm2
   27d8d:	66 0f 28 dd          	movapd %xmm5,%xmm3
   27d91:	f2 0f c2 dd 03       	cmpunordsd %xmm5,%xmm3
   27d96:	66 0f 28 cb          	movapd %xmm3,%xmm1
   27d9a:	66 0f 54 ca          	andpd  %xmm2,%xmm1
   27d9e:	66 0f 29 ac 24 e0 01 	movapd %xmm5,0x1e0(%rsp)
   27da5:	00 00 
   27da7:	f2 0f 5f d5          	maxsd  %xmm5,%xmm2
   27dab:	66 0f 55 da          	andnpd %xmm2,%xmm3
   27daf:	66 0f 56 d9          	orpd   %xmm1,%xmm3
   27db3:	f2 0f 58 d8          	addsd  %xmm0,%xmm3
   27db7:	f2 0f 59 1d 89 4f fe 	mulsd  -0x1b077(%rip),%xmm3        # cd48 <GCC_except_table0+0xc0>
   27dbe:	ff 
   27dbf:	66 0f 14 db          	unpcklpd %xmm3,%xmm3
   27dc3:	66 0f 29 5c 24 60    	movapd %xmm3,0x60(%rsp)
   27dc9:	66 0f 58 1d 5f 52 fe 	addpd  -0x1ada1(%rip),%xmm3        # d030 <GCC_except_table0+0x3a8>
   27dd0:	ff 
   27dd1:	66 0f 28 05 c7 51 fe 	movapd -0x1ae39(%rip),%xmm0        # cfa0 <GCC_except_table0+0x318>
   27dd8:	ff 
   27dd9:	66 0f 59 d8          	mulpd  %xmm0,%xmm3
   27ddd:	66 0f 28 05 db 54 fe 	movapd -0x1ab25(%rip),%xmm0        # d2c0 <GCC_except_table0+0x638>
   27de4:	ff 
   27de5:	66 0f 5e d8          	divpd  %xmm0,%xmm3
   27de9:	66 0f 59 db          	mulpd  %xmm3,%xmm3
   27ded:	66 0f 28 05 3b 56 fe 	movapd -0x1a9c5(%rip),%xmm0        # d430 <GCC_except_table0+0x7a8>
   27df4:	ff 
   27df5:	66 0f 59 d8          	mulpd  %xmm0,%xmm3
   27df9:	66 0f 29 9c 24 80 00 	movapd %xmm3,0x80(%rsp)
   27e00:	00 00 
   27e02:	66 0f 28 c3          	movapd %xmm3,%xmm0
   27e06:	4c 8b 35 f3 bc 04 00 	mov    0x4bcf3(%rip),%r14        # 73b00 <exp@GLIBC_2.29>
   27e0d:	48 89 d3             	mov    %rdx,%rbx
   27e10:	41 ff d6             	call   *%r14
   27e13:	66 0f 29 44 24 50    	movapd %xmm0,0x50(%rsp)
   27e19:	0f 28 84 24 80 00 00 	movaps 0x80(%rsp),%xmm0
   27e20:	00 
   27e21:	0f 12 c0             	movhlps %xmm0,%xmm0
   27e24:	41 ff d6             	call   *%r14
   27e27:	0f 29 84 24 80 01 00 	movaps %xmm0,0x180(%rsp)
   27e2e:	00 
   27e2f:	0f 28 4c 24 50       	movaps 0x50(%rsp),%xmm1
   27e34:	0f 16 c8             	movlhps %xmm0,%xmm1
   27e37:	0f 29 4c 24 50       	movaps %xmm1,0x50(%rsp)
   27e3c:	0f 29 8c 24 20 02 00 	movaps %xmm1,0x220(%rsp)
   27e43:	00 
   27e44:	66 0f 28 44 24 60    	movapd 0x60(%rsp),%xmm0
   27e4a:	66 0f 58 05 be 52 fe 	addpd  -0x1ad42(%rip),%xmm0        # d110 <GCC_except_table0+0x488>
   27e51:	ff 
   27e52:	66 0f 59 05 46 51 fe 	mulpd  -0x1aeba(%rip),%xmm0        # cfa0 <GCC_except_table0+0x318>
   27e59:	ff 
   27e5a:	66 0f 5e 05 5e 54 fe 	divpd  -0x1aba2(%rip),%xmm0        # d2c0 <GCC_except_table0+0x638>
   27e61:	ff 
   27e62:	66 0f 59 c0          	mulpd  %xmm0,%xmm0
   27e66:	66 0f 59 05 c2 55 fe 	mulpd  -0x1aa3e(%rip),%xmm0        # d430 <GCC_except_table0+0x7a8>
   27e6d:	ff 
   27e6e:	66 0f 29 84 24 10 01 	movapd %xmm0,0x110(%rsp)
   27e75:	00 00 
   27e77:	41 ff d6             	call   *%r14
   27e7a:	66 0f 29 84 24 80 00 	movapd %xmm0,0x80(%rsp)
   27e81:	00 00 
   27e83:	0f 28 84 24 10 01 00 	movaps 0x110(%rsp),%xmm0
   27e8a:	00 
   27e8b:	0f 12 c0             	movhlps %xmm0,%xmm0
   27e8e:	41 ff d6             	call   *%r14
   27e91:	0f 29 84 24 70 01 00 	movaps %xmm0,0x170(%rsp)
   27e98:	00 
   27e99:	0f 28 8c 24 80 00 00 	movaps 0x80(%rsp),%xmm1
   27ea0:	00 
   27ea1:	0f 16 c8             	movlhps %xmm0,%xmm1
   27ea4:	0f 29 8c 24 80 00 00 	movaps %xmm1,0x80(%rsp)
   27eab:	00 
   27eac:	0f 29 8c 24 30 02 00 	movaps %xmm1,0x230(%rsp)
   27eb3:	00 
   27eb4:	66 0f 28 44 24 60    	movapd 0x60(%rsp),%xmm0
   27eba:	66 0f 58 05 1e 50 fe 	addpd  -0x1afe2(%rip),%xmm0        # cee0 <GCC_except_table0+0x258>
   27ec1:	ff 
   27ec2:	66 0f 59 05 d6 50 fe 	mulpd  -0x1af2a(%rip),%xmm0        # cfa0 <GCC_except_table0+0x318>
   27ec9:	ff 
   27eca:	66 0f 5e 05 ee 53 fe 	divpd  -0x1ac12(%rip),%xmm0        # d2c0 <GCC_except_table0+0x638>
   27ed1:	ff 
   27ed2:	66 0f 59 c0          	mulpd  %xmm0,%xmm0
   27ed6:	66 0f 59 05 52 55 fe 	mulpd  -0x1aaae(%rip),%xmm0        # d430 <GCC_except_table0+0x7a8>
   27edd:	ff 
   27ede:	66 0f 29 84 24 30 01 	movapd %xmm0,0x130(%rsp)
   27ee5:	00 00 
   27ee7:	41 ff d6             	call   *%r14
   27eea:	66 0f 29 84 24 10 01 	movapd %xmm0,0x110(%rsp)
   27ef1:	00 00 
   27ef3:	0f 28 84 24 30 01 00 	movaps 0x130(%rsp),%xmm0
   27efa:	00 
   27efb:	0f 12 c0             	movhlps %xmm0,%xmm0
   27efe:	41 ff d6             	call   *%r14
   27f01:	0f 29 84 24 30 01 00 	movaps %xmm0,0x130(%rsp)
   27f08:	00 
   27f09:	0f 28 8c 24 10 01 00 	movaps 0x110(%rsp),%xmm1
   27f10:	00 
   27f11:	0f 16 c8             	movlhps %xmm0,%xmm1
   27f14:	0f 29 8c 24 10 01 00 	movaps %xmm1,0x110(%rsp)
   27f1b:	00 
   27f1c:	0f 29 8c 24 40 02 00 	movaps %xmm1,0x240(%rsp)
   27f23:	00 
   27f24:	66 0f 28 44 24 60    	movapd 0x60(%rsp),%xmm0
   27f2a:	66 0f 58 05 de 52 fe 	addpd  -0x1ad22(%rip),%xmm0        # d210 <GCC_except_table0+0x588>
   27f31:	ff 
   27f32:	66 0f 59 05 66 50 fe 	mulpd  -0x1af9a(%rip),%xmm0        # cfa0 <GCC_except_table0+0x318>
   27f39:	ff 
   27f3a:	66 0f 5e 05 7e 53 fe 	divpd  -0x1ac82(%rip),%xmm0        # d2c0 <GCC_except_table0+0x638>
   27f41:	ff 
   27f42:	66 0f 59 c0          	mulpd  %xmm0,%xmm0
   27f46:	66 0f 59 05 e2 54 fe 	mulpd  -0x1ab1e(%rip),%xmm0        # d430 <GCC_except_table0+0x7a8>
   27f4d:	ff 
   27f4e:	66 0f 29 44 24 60    	movapd %xmm0,0x60(%rsp)
   27f54:	41 ff d6             	call   *%r14
   27f57:	66 0f 29 84 24 60 01 	movapd %xmm0,0x160(%rsp)
   27f5e:	00 00 
   27f60:	66 0f 28 44 24 60    	movapd 0x60(%rsp),%xmm0
   27f66:	66 0f 15 c0          	unpckhpd %xmm0,%xmm0
   27f6a:	41 ff d6             	call   *%r14
   27f6d:	48 89 da             	mov    %rbx,%rdx
   27f70:	66 0f 28 8c 24 60 01 	movapd 0x160(%rsp),%xmm1
   27f77:	00 00 
   27f79:	66 0f 14 c8          	unpcklpd %xmm0,%xmm1
   27f7d:	66 0f 29 8c 24 50 02 	movapd %xmm1,0x250(%rsp)
   27f84:	00 00 
   27f86:	66 0f 28 d1          	movapd %xmm1,%xmm2
   27f8a:	66 0f 28 4c 24 50    	movapd 0x50(%rsp),%xmm1
   27f90:	f2 0f 58 8c 24 80 01 	addsd  0x180(%rsp),%xmm1
   27f97:	00 00 
   27f99:	f2 0f 58 8c 24 80 00 	addsd  0x80(%rsp),%xmm1
   27fa0:	00 00 
   27fa2:	f2 0f 58 8c 24 70 01 	addsd  0x170(%rsp),%xmm1
   27fa9:	00 00 
   27fab:	f2 0f 58 8c 24 10 01 	addsd  0x110(%rsp),%xmm1
   27fb2:	00 00 
   27fb4:	f2 0f 58 8c 24 30 01 	addsd  0x130(%rsp),%xmm1
   27fbb:	00 00 
   27fbd:	f2 0f 58 ca          	addsd  %xmm2,%xmm1
   27fc1:	f2 0f 58 c8          	addsd  %xmm0,%xmm1
   27fc5:	66 0f 57 c0          	xorpd  %xmm0,%xmm0
   27fc9:	66 0f 29 4c 24 50    	movapd %xmm1,0x50(%rsp)
   27fcf:	66 0f 2e c1          	ucomisd %xmm1,%xmm0
   27fd3:	4d 89 fe             	mov    %r15,%r14
   27fd6:	48 8b 9c 24 90 00 00 	mov    0x90(%rsp),%rbx
   27fdd:	00 
   27fde:	0f 83 6c fc ff ff    	jae    27c50 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x280>
   27fe4:	b9 0c 00 00 00       	mov    $0xc,%ecx
   27fe9:	48 8d 7c 24 28       	lea    0x28(%rsp),%rdi
   27fee:	48 8b 74 24 70       	mov    0x70(%rsp),%rsi
   27ff3:	4c 89 f2             	mov    %r14,%rdx
   27ff6:	e8 85 e7 ff ff       	call   26780 <xdremux_engine::apple_photographic_styles::spotlight_weights>
   27ffb:	f3 0f 10 84 24 58 01 	movss  0x158(%rsp),%xmm0
   28002:	00 00 
   28004:	f3 0f 5a c0          	cvtss2sd %xmm0,%xmm0
   28008:	f3 0f 10 8c 24 50 01 	movss  0x150(%rsp),%xmm1
   2800f:	00 00 
   28011:	f3 0f 5a c9          	cvtss2sd %xmm1,%xmm1
   28015:	f3 0f 10 94 24 48 01 	movss  0x148(%rsp),%xmm2
   2801c:	00 00 
   2801e:	f3 0f 5a d2          	cvtss2sd %xmm2,%xmm2
   28022:	f2 0f 10 64 24 30    	movsd  0x30(%rsp),%xmm4
   28028:	48 8b 44 24 28       	mov    0x28(%rsp),%rax
   2802d:	48 89 84 24 80 00 00 	mov    %rax,0x80(%rsp)
   28034:	00 
   28035:	48 8d 0c c0          	lea    (%rax,%rax,8),%rcx
   28039:	f2 0f 5c 84 24 f0 01 	subsd  0x1f0(%rsp),%xmm0
   28040:	00 00 
   28042:	f2 0f 5c 8c 24 d0 01 	subsd  0x1d0(%rsp),%xmm1
   28049:	00 00 
   2804b:	f2 0f 5c 94 24 e0 01 	subsd  0x1e0(%rsp),%xmm2
   28052:	00 00 
   28054:	f2 0f 10 5c 24 40    	movsd  0x40(%rsp),%xmm3
   2805a:	48 8b 44 24 38       	mov    0x38(%rsp),%rax
   2805f:	48 89 44 24 60       	mov    %rax,0x60(%rsp)
   28064:	66 0f 28 ec          	movapd %xmm4,%xmm5
   28068:	f2 0f 59 ac 24 28 01 	mulsd  0x128(%rsp),%xmm5
   2806f:	00 00 
   28071:	66 0f 57 f6          	xorpd  %xmm6,%xmm6
   28075:	66 0f 2e f5          	ucomisd %xmm5,%xmm6
   28079:	0f 83 a1 01 00 00    	jae    28220 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x850>
   2807f:	48 8b 84 24 c0 01 00 	mov    0x1c0(%rsp),%rax
   28086:	00 
   28087:	48 8d 14 08          	lea    (%rax,%rcx,1),%rdx
   2808b:	48 c1 e2 03          	shl    $0x3,%rdx
   2808f:	48 8b b4 24 80 00 00 	mov    0x80(%rsp),%rsi
   28096:	00 
   28097:	48 69 c6 20 1c 00 00 	imul   $0x1c20,%rsi,%rax
   2809e:	48 03 84 24 b8 01 00 	add    0x1b8(%rsp),%rax
   280a5:	00 
   280a6:	4c 69 d6 00 e1 00 00 	imul   $0xe100,%rsi,%r10
   280ad:	4c 03 94 24 b0 01 00 	add    0x1b0(%rsp),%r10
   280b4:	00 
   280b5:	45 31 db             	xor    %r11d,%r11d
   280b8:	31 db                	xor    %ebx,%ebx
   280ba:	eb 20                	jmp    280dc <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x70c>
   280bc:	0f 1f 40 00          	nopl   0x0(%rax)
   280c0:	49 83 c3 08          	add    $0x8,%r11
   280c4:	48 ff c3             	inc    %rbx
   280c7:	48 83 c0 64          	add    $0x64,%rax
   280cb:	49 81 c2 20 03 00 00 	add    $0x320,%r10
   280d2:	49 83 fb 40          	cmp    $0x40,%r11
   280d6:	0f 84 44 01 00 00    	je     28220 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x850>
   280dc:	f2 42 0f 10 b4 1c 20 	movsd  0x220(%rsp,%r11,1),%xmm6
   280e3:	02 00 00 
   280e6:	f2 0f 59 f5          	mulsd  %xmm5,%xmm6
   280ea:	f2 0f 5e 74 24 50    	divsd  0x50(%rsp),%xmm6
   280f0:	66 0f 57 ff          	xorpd  %xmm7,%xmm7
   280f4:	66 0f 2e fe          	ucomisd %xmm6,%xmm7
   280f8:	73 c6                	jae    280c0 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x6f0>
   280fa:	4c 8d 34 13          	lea    (%rbx,%rdx,1),%r14
   280fe:	4b 8d 34 36          	lea    (%r14,%r14,1),%rsi
   28102:	49 c1 e6 05          	shl    $0x5,%r14
   28106:	49 29 f6             	sub    %rsi,%r14
   28109:	4c 89 d5             	mov    %r10,%rbp
   2810c:	49 89 c1             	mov    %rax,%r9
   2810f:	4c 8d a4 24 a0 00 00 	lea    0xa0(%rsp),%r12
   28116:	00 
   28117:	31 f6                	xor    %esi,%esi
   28119:	0f 1f 80 00 00 00 00 	nopl   0x0(%rax)
   28120:	4c 8d 7e 01          	lea    0x1(%rsi),%r15
   28124:	45 31 c0             	xor    %r8d,%r8d
   28127:	66 0f 1f 84 00 00 00 	nopw   0x0(%rax,%rax,1)
   2812e:	00 00 
   28130:	4b 8d 3c 01          	lea    (%r9,%r8,1),%rdi
   28134:	48 81 ff 7f 51 01 00 	cmp    $0x1517f,%rdi
   2813b:	0f 87 14 06 00 00    	ja     28755 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0xd85>
   28141:	f2 0f 10 bc f4 a0 00 	movsd  0xa0(%rsp,%rsi,8),%xmm7
   28148:	00 00 
   2814a:	f2 0f 59 fe          	mulsd  %xmm6,%xmm7
   2814e:	f2 47 0f 10 04 c4    	movsd  (%r12,%r8,8),%xmm8
   28154:	f2 44 0f 59 c7       	mulsd  %xmm7,%xmm8
   28159:	f2 46 0f 58 44 c5 00 	addsd  0x0(%rbp,%r8,8),%xmm8
   28160:	f2 46 0f 11 44 c5 00 	movsd  %xmm8,0x0(%rbp,%r8,8)
   28167:	4a 8d 3c 06          	lea    (%rsi,%r8,1),%rdi
   2816b:	48 ff c7             	inc    %rdi
   2816e:	49 ff c0             	inc    %r8
   28171:	48 83 ff 0a          	cmp    $0xa,%rdi
   28175:	75 b9                	jne    28130 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x760>
   28177:	4c 8d 04 76          	lea    (%rsi,%rsi,2),%r8
   2817b:	4d 01 f0             	add    %r14,%r8
   2817e:	48 8b 74 24 20       	mov    0x20(%rsp),%rsi
   28183:	49 39 f0             	cmp    %rsi,%r8
   28186:	0f 83 07 10 00 00    	jae    29193 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x17c3>
   2818c:	48 8b 74 24 18       	mov    0x18(%rsp),%rsi
   28191:	66 44 0f 28 c0       	movapd %xmm0,%xmm8
   28196:	f2 44 0f 59 c7       	mulsd  %xmm7,%xmm8
   2819b:	f2 46 0f 58 04 c6    	addsd  (%rsi,%r8,8),%xmm8
   281a1:	f2 46 0f 11 04 c6    	movsd  %xmm8,(%rsi,%r8,8)
   281a7:	49 8d 78 01          	lea    0x1(%r8),%rdi
   281ab:	48 8b 74 24 20       	mov    0x20(%rsp),%rsi
   281b0:	48 39 f7             	cmp    %rsi,%rdi
   281b3:	0f 83 ec 0f 00 00    	jae    291a5 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x17d5>
   281b9:	48 8b 74 24 18       	mov    0x18(%rsp),%rsi
   281be:	66 44 0f 28 c1       	movapd %xmm1,%xmm8
   281c3:	f2 44 0f 59 c7       	mulsd  %xmm7,%xmm8
   281c8:	f2 46 0f 58 44 c6 08 	addsd  0x8(%rsi,%r8,8),%xmm8
   281cf:	f2 46 0f 11 44 c6 08 	movsd  %xmm8,0x8(%rsi,%r8,8)
   281d6:	49 8d 78 02          	lea    0x2(%r8),%rdi
   281da:	48 8b 74 24 20       	mov    0x20(%rsp),%rsi
   281df:	48 39 f7             	cmp    %rsi,%rdi
   281e2:	0f 83 bd 0f 00 00    	jae    291a5 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x17d5>
   281e8:	48 8b 74 24 18       	mov    0x18(%rsp),%rsi
   281ed:	f2 0f 59 fa          	mulsd  %xmm2,%xmm7
   281f1:	f2 42 0f 58 7c c6 10 	addsd  0x10(%rsi,%r8,8),%xmm7
   281f8:	f2 42 0f 11 7c c6 10 	movsd  %xmm7,0x10(%rsi,%r8,8)
   281ff:	49 83 c4 08          	add    $0x8,%r12
   28203:	49 83 c1 0b          	add    $0xb,%r9
   28207:	48 83 c5 58          	add    $0x58,%rbp
   2820b:	4c 89 fe             	mov    %r15,%rsi
   2820e:	49 83 ff 0a          	cmp    $0xa,%r15
   28212:	0f 85 08 ff ff ff    	jne    28120 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x750>
   28218:	e9 a3 fe ff ff       	jmp    280c0 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x6f0>
   2821d:	0f 1f 00             	nopl   (%rax)
   28220:	48 8b 44 24 60       	mov    0x60(%rsp),%rax
   28225:	48 8d 14 c0          	lea    (%rax,%rax,8),%rdx
   28229:	66 0f 28 eb          	movapd %xmm3,%xmm5
   2822d:	f2 0f 59 ac 24 28 01 	mulsd  0x128(%rsp),%xmm5
   28234:	00 00 
   28236:	66 0f 57 f6          	xorpd  %xmm6,%xmm6
   2823a:	66 0f 2e f5          	ucomisd %xmm5,%xmm6
   2823e:	0f 83 9c 01 00 00    	jae    283e0 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0xa10>
   28244:	48 8b 84 24 c0 01 00 	mov    0x1c0(%rsp),%rax
   2824b:	00 
   2824c:	4c 8d 0c 10          	lea    (%rax,%rdx,1),%r9
   28250:	49 c1 e1 03          	shl    $0x3,%r9
   28254:	48 8b 44 24 60       	mov    0x60(%rsp),%rax
   28259:	4c 69 d0 20 1c 00 00 	imul   $0x1c20,%rax,%r10
   28260:	4c 03 94 24 b8 01 00 	add    0x1b8(%rsp),%r10
   28267:	00 
   28268:	4c 69 d8 00 e1 00 00 	imul   $0xe100,%rax,%r11
   2826f:	4c 03 9c 24 b0 01 00 	add    0x1b0(%rsp),%r11
   28276:	00 
   28277:	31 db                	xor    %ebx,%ebx
   28279:	45 31 f6             	xor    %r14d,%r14d
   2827c:	eb 1e                	jmp    2829c <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x8cc>
   2827e:	66 90                	xchg   %ax,%ax
   28280:	48 83 c3 08          	add    $0x8,%rbx
   28284:	49 ff c6             	inc    %r14
   28287:	49 83 c2 64          	add    $0x64,%r10
   2828b:	49 81 c3 20 03 00 00 	add    $0x320,%r11
   28292:	48 83 fb 40          	cmp    $0x40,%rbx
   28296:	0f 84 44 01 00 00    	je     283e0 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0xa10>
   2829c:	f2 0f 10 b4 1c 20 02 	movsd  0x220(%rsp,%rbx,1),%xmm6
   282a3:	00 00 
   282a5:	f2 0f 59 f5          	mulsd  %xmm5,%xmm6
   282a9:	f2 0f 5e 74 24 50    	divsd  0x50(%rsp),%xmm6
   282af:	66 0f 57 ff          	xorpd  %xmm7,%xmm7
   282b3:	66 0f 2e fe          	ucomisd %xmm6,%xmm7
   282b7:	73 c7                	jae    28280 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x8b0>
   282b9:	4b 8d 2c 0e          	lea    (%r14,%r9,1),%rbp
   282bd:	48 8d 04 6d 00 00 00 	lea    0x0(,%rbp,2),%rax
   282c4:	00 
   282c5:	48 c1 e5 05          	shl    $0x5,%rbp
   282c9:	48 29 c5             	sub    %rax,%rbp
   282cc:	4c 89 d8             	mov    %r11,%rax
   282cf:	4d 89 d4             	mov    %r10,%r12
   282d2:	4c 8d bc 24 a0 00 00 	lea    0xa0(%rsp),%r15
   282d9:	00 
   282da:	31 f6                	xor    %esi,%esi
   282dc:	0f 1f 40 00          	nopl   0x0(%rax)
   282e0:	4c 8d 46 01          	lea    0x1(%rsi),%r8
   282e4:	45 31 ed             	xor    %r13d,%r13d
   282e7:	66 0f 1f 84 00 00 00 	nopw   0x0(%rax,%rax,1)
   282ee:	00 00 
   282f0:	4b 8d 3c 2c          	lea    (%r12,%r13,1),%rdi
   282f4:	48 81 ff 7f 51 01 00 	cmp    $0x1517f,%rdi
   282fb:	0f 87 54 04 00 00    	ja     28755 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0xd85>
   28301:	f2 0f 10 bc f4 a0 00 	movsd  0xa0(%rsp,%rsi,8),%xmm7
   28308:	00 00 
   2830a:	f2 0f 59 fe          	mulsd  %xmm6,%xmm7
   2830e:	f2 47 0f 10 04 ef    	movsd  (%r15,%r13,8),%xmm8
   28314:	f2 44 0f 59 c7       	mulsd  %xmm7,%xmm8
   28319:	f2 46 0f 58 04 e8    	addsd  (%rax,%r13,8),%xmm8
   2831f:	f2 46 0f 11 04 e8    	movsd  %xmm8,(%rax,%r13,8)
   28325:	4a 8d 3c 2e          	lea    (%rsi,%r13,1),%rdi
   28329:	48 ff c7             	inc    %rdi
   2832c:	49 ff c5             	inc    %r13
   2832f:	48 83 ff 0a          	cmp    $0xa,%rdi
   28333:	75 bb                	jne    282f0 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x920>
   28335:	4c 8d 2c 76          	lea    (%rsi,%rsi,2),%r13
   28339:	49 01 ed             	add    %rbp,%r13
   2833c:	48 8b 74 24 20       	mov    0x20(%rsp),%rsi
   28341:	49 39 f5             	cmp    %rsi,%r13
   28344:	0f 83 4e 0e 00 00    	jae    29198 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x17c8>
   2834a:	48 8b 74 24 18       	mov    0x18(%rsp),%rsi
   2834f:	66 44 0f 28 c0       	movapd %xmm0,%xmm8
   28354:	f2 44 0f 59 c7       	mulsd  %xmm7,%xmm8
   28359:	f2 46 0f 58 04 ee    	addsd  (%rsi,%r13,8),%xmm8
   2835f:	f2 46 0f 11 04 ee    	movsd  %xmm8,(%rsi,%r13,8)
   28365:	49 8d 7d 01          	lea    0x1(%r13),%rdi
   28369:	48 8b 74 24 20       	mov    0x20(%rsp),%rsi
   2836e:	48 39 f7             	cmp    %rsi,%rdi
   28371:	0f 83 2e 0e 00 00    	jae    291a5 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x17d5>
   28377:	48 8b 74 24 18       	mov    0x18(%rsp),%rsi
   2837c:	66 44 0f 28 c1       	movapd %xmm1,%xmm8
   28381:	f2 44 0f 59 c7       	mulsd  %xmm7,%xmm8
   28386:	f2 46 0f 58 44 ee 08 	addsd  0x8(%rsi,%r13,8),%xmm8
   2838d:	f2 46 0f 11 44 ee 08 	movsd  %xmm8,0x8(%rsi,%r13,8)
   28394:	49 8d 7d 02          	lea    0x2(%r13),%rdi
   28398:	48 8b 74 24 20       	mov    0x20(%rsp),%rsi
   2839d:	48 39 f7             	cmp    %rsi,%rdi
   283a0:	0f 83 ff 0d 00 00    	jae    291a5 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x17d5>
   283a6:	48 8b 74 24 18       	mov    0x18(%rsp),%rsi
   283ab:	f2 0f 59 fa          	mulsd  %xmm2,%xmm7
   283af:	f2 42 0f 58 7c ee 10 	addsd  0x10(%rsi,%r13,8),%xmm7
   283b6:	f2 42 0f 11 7c ee 10 	movsd  %xmm7,0x10(%rsi,%r13,8)
   283bd:	49 83 c7 08          	add    $0x8,%r15
   283c1:	49 83 c4 0b          	add    $0xb,%r12
   283c5:	48 83 c0 58          	add    $0x58,%rax
   283c9:	4c 89 c6             	mov    %r8,%rsi
   283cc:	49 83 f8 0a          	cmp    $0xa,%r8
   283d0:	0f 85 0a ff ff ff    	jne    282e0 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x910>
   283d6:	e9 a5 fe ff ff       	jmp    28280 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x8b0>
   283db:	0f 1f 44 00 00       	nopl   0x0(%rax,%rax,1)
   283e0:	f2 0f 59 a4 24 c8 01 	mulsd  0x1c8(%rsp),%xmm4
   283e7:	00 00 
   283e9:	66 0f 57 ed          	xorpd  %xmm5,%xmm5
   283ed:	66 0f 2e ec          	ucomisd %xmm4,%xmm5
   283f1:	4c 8d ac 24 f0 00 00 	lea    0xf0(%rsp),%r13
   283f8:	00 
   283f9:	48 8b 6c 24 08       	mov    0x8(%rsp),%rbp
   283fe:	0f 83 9c 01 00 00    	jae    285a0 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0xbd0>
   28404:	48 03 8c 24 a8 01 00 	add    0x1a8(%rsp),%rcx
   2840b:	00 
   2840c:	48 c1 e1 03          	shl    $0x3,%rcx
   28410:	48 8b 84 24 80 00 00 	mov    0x80(%rsp),%rax
   28417:	00 
   28418:	4c 69 c8 20 1c 00 00 	imul   $0x1c20,%rax,%r9
   2841f:	4c 03 8c 24 a0 01 00 	add    0x1a0(%rsp),%r9
   28426:	00 
   28427:	4c 69 c0 00 e1 00 00 	imul   $0xe100,%rax,%r8
   2842e:	4c 03 84 24 98 01 00 	add    0x198(%rsp),%r8
   28435:	00 
   28436:	45 31 d2             	xor    %r10d,%r10d
   28439:	45 31 db             	xor    %r11d,%r11d
   2843c:	eb 1e                	jmp    2845c <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0xa8c>
   2843e:	66 90                	xchg   %ax,%ax
   28440:	49 83 c2 08          	add    $0x8,%r10
   28444:	49 ff c3             	inc    %r11
   28447:	49 83 c1 64          	add    $0x64,%r9
   2844b:	49 81 c0 20 03 00 00 	add    $0x320,%r8
   28452:	49 83 fa 40          	cmp    $0x40,%r10
   28456:	0f 84 44 01 00 00    	je     285a0 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0xbd0>
   2845c:	f2 42 0f 10 ac 14 20 	movsd  0x220(%rsp,%r10,1),%xmm5
   28463:	02 00 00 
   28466:	f2 0f 59 ec          	mulsd  %xmm4,%xmm5
   2846a:	f2 0f 5e 6c 24 50    	divsd  0x50(%rsp),%xmm5
   28470:	66 0f 57 f6          	xorpd  %xmm6,%xmm6
   28474:	66 0f 2e f5          	ucomisd %xmm5,%xmm6
   28478:	73 c6                	jae    28440 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0xa70>
   2847a:	49 8d 1c 0b          	lea    (%r11,%rcx,1),%rbx
   2847e:	48 8d 04 1b          	lea    (%rbx,%rbx,1),%rax
   28482:	48 c1 e3 05          	shl    $0x5,%rbx
   28486:	48 29 c3             	sub    %rax,%rbx
   28489:	4c 89 c0             	mov    %r8,%rax
   2848c:	4d 89 ce             	mov    %r9,%r14
   2848f:	4c 8d a4 24 a0 00 00 	lea    0xa0(%rsp),%r12
   28496:	00 
   28497:	31 f6                	xor    %esi,%esi
   28499:	0f 1f 80 00 00 00 00 	nopl   0x0(%rax)
   284a0:	4c 8d 7e 01          	lea    0x1(%rsi),%r15
   284a4:	45 31 ed             	xor    %r13d,%r13d
   284a7:	66 0f 1f 84 00 00 00 	nopw   0x0(%rax,%rax,1)
   284ae:	00 00 
   284b0:	4b 8d 3c 2e          	lea    (%r14,%r13,1),%rdi
   284b4:	48 81 ff 7f 51 01 00 	cmp    $0x1517f,%rdi
   284bb:	0f 87 94 02 00 00    	ja     28755 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0xd85>
   284c1:	f2 0f 10 b4 f4 a0 00 	movsd  0xa0(%rsp,%rsi,8),%xmm6
   284c8:	00 00 
   284ca:	f2 0f 59 f5          	mulsd  %xmm5,%xmm6
   284ce:	f2 43 0f 10 3c ec    	movsd  (%r12,%r13,8),%xmm7
   284d4:	f2 0f 59 fe          	mulsd  %xmm6,%xmm7
   284d8:	f2 42 0f 58 3c e8    	addsd  (%rax,%r13,8),%xmm7
   284de:	f2 42 0f 11 3c e8    	movsd  %xmm7,(%rax,%r13,8)
   284e4:	4a 8d 3c 2e          	lea    (%rsi,%r13,1),%rdi
   284e8:	48 ff c7             	inc    %rdi
   284eb:	49 ff c5             	inc    %r13
   284ee:	48 83 ff 0a          	cmp    $0xa,%rdi
   284f2:	75 bc                	jne    284b0 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0xae0>
   284f4:	48 8d 2c 76          	lea    (%rsi,%rsi,2),%rbp
   284f8:	48 01 dd             	add    %rbx,%rbp
   284fb:	48 8b 74 24 20       	mov    0x20(%rsp),%rsi
   28500:	48 39 f5             	cmp    %rsi,%rbp
   28503:	0f 83 94 0c 00 00    	jae    2919d <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x17cd>
   28509:	48 8b 74 24 18       	mov    0x18(%rsp),%rsi
   2850e:	66 0f 28 f8          	movapd %xmm0,%xmm7
   28512:	f2 0f 59 fe          	mulsd  %xmm6,%xmm7
   28516:	f2 0f 58 3c ee       	addsd  (%rsi,%rbp,8),%xmm7
   2851b:	f2 0f 11 3c ee       	movsd  %xmm7,(%rsi,%rbp,8)
   28520:	48 8d 7d 01          	lea    0x1(%rbp),%rdi
   28524:	48 8b 74 24 20       	mov    0x20(%rsp),%rsi
   28529:	48 39 f7             	cmp    %rsi,%rdi
   2852c:	0f 83 73 0c 00 00    	jae    291a5 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x17d5>
   28532:	48 8b 74 24 18       	mov    0x18(%rsp),%rsi
   28537:	66 0f 28 f9          	movapd %xmm1,%xmm7
   2853b:	f2 0f 59 fe          	mulsd  %xmm6,%xmm7
   2853f:	f2 0f 58 7c ee 08    	addsd  0x8(%rsi,%rbp,8),%xmm7
   28545:	f2 0f 11 7c ee 08    	movsd  %xmm7,0x8(%rsi,%rbp,8)
   2854b:	48 8d 7d 02          	lea    0x2(%rbp),%rdi
   2854f:	48 8b 74 24 20       	mov    0x20(%rsp),%rsi
   28554:	48 39 f7             	cmp    %rsi,%rdi
   28557:	0f 83 48 0c 00 00    	jae    291a5 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x17d5>
   2855d:	48 8b 74 24 18       	mov    0x18(%rsp),%rsi
   28562:	f2 0f 59 f2          	mulsd  %xmm2,%xmm6
   28566:	f2 0f 58 74 ee 10    	addsd  0x10(%rsi,%rbp,8),%xmm6
   2856c:	f2 0f 11 74 ee 10    	movsd  %xmm6,0x10(%rsi,%rbp,8)
   28572:	49 83 c4 08          	add    $0x8,%r12
   28576:	49 83 c6 0b          	add    $0xb,%r14
   2857a:	48 83 c0 58          	add    $0x58,%rax
   2857e:	4c 89 fe             	mov    %r15,%rsi
   28581:	49 83 ff 0a          	cmp    $0xa,%r15
   28585:	4c 8d ac 24 f0 00 00 	lea    0xf0(%rsp),%r13
   2858c:	00 
   2858d:	48 8b 6c 24 08       	mov    0x8(%rsp),%rbp
   28592:	0f 85 08 ff ff ff    	jne    284a0 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0xad0>
   28598:	e9 a3 fe ff ff       	jmp    28440 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0xa70>
   2859d:	0f 1f 00             	nopl   (%rax)
   285a0:	f2 0f 59 9c 24 c8 01 	mulsd  0x1c8(%rsp),%xmm3
   285a7:	00 00 
   285a9:	66 0f 57 e4          	xorpd  %xmm4,%xmm4
   285ad:	66 0f 2e e3          	ucomisd %xmm3,%xmm4
   285b1:	0f 83 79 f6 ff ff    	jae    27c30 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x260>
   285b7:	48 03 94 24 a8 01 00 	add    0x1a8(%rsp),%rdx
   285be:	00 
   285bf:	48 c1 e2 03          	shl    $0x3,%rdx
   285c3:	48 8b 44 24 60       	mov    0x60(%rsp),%rax
   285c8:	48 69 c8 20 1c 00 00 	imul   $0x1c20,%rax,%rcx
   285cf:	48 03 8c 24 a0 01 00 	add    0x1a0(%rsp),%rcx
   285d6:	00 
   285d7:	48 69 c0 00 e1 00 00 	imul   $0xe100,%rax,%rax
   285de:	48 03 84 24 98 01 00 	add    0x198(%rsp),%rax
   285e5:	00 
   285e6:	45 31 c0             	xor    %r8d,%r8d
   285e9:	45 31 c9             	xor    %r9d,%r9d
   285ec:	eb 1d                	jmp    2860b <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0xc3b>
   285ee:	66 90                	xchg   %ax,%ax
   285f0:	49 83 c0 08          	add    $0x8,%r8
   285f4:	49 ff c1             	inc    %r9
   285f7:	48 83 c1 64          	add    $0x64,%rcx
   285fb:	48 05 20 03 00 00    	add    $0x320,%rax
   28601:	49 83 f8 40          	cmp    $0x40,%r8
   28605:	0f 84 25 f6 ff ff    	je     27c30 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x260>
   2860b:	f2 42 0f 10 a4 04 20 	movsd  0x220(%rsp,%r8,1),%xmm4
   28612:	02 00 00 
   28615:	f2 0f 59 e3          	mulsd  %xmm3,%xmm4
   28619:	f2 0f 5e 64 24 50    	divsd  0x50(%rsp),%xmm4
   2861f:	66 0f 57 ed          	xorpd  %xmm5,%xmm5
   28623:	66 0f 2e ec          	ucomisd %xmm4,%xmm5
   28627:	73 c7                	jae    285f0 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0xc20>
   28629:	4d 8d 14 11          	lea    (%r9,%rdx,1),%r10
   2862d:	4b 8d 34 12          	lea    (%r10,%r10,1),%rsi
   28631:	49 c1 e2 05          	shl    $0x5,%r10
   28635:	49 29 f2             	sub    %rsi,%r10
   28638:	49 89 c3             	mov    %rax,%r11
   2863b:	48 89 cb             	mov    %rcx,%rbx
   2863e:	4c 8d b4 24 a0 00 00 	lea    0xa0(%rsp),%r14
   28645:	00 
   28646:	31 f6                	xor    %esi,%esi
   28648:	0f 1f 84 00 00 00 00 	nopl   0x0(%rax,%rax,1)
   2864f:	00 
   28650:	4c 8d 7e 01          	lea    0x1(%rsi),%r15
   28654:	45 31 e4             	xor    %r12d,%r12d
   28657:	66 0f 1f 84 00 00 00 	nopw   0x0(%rax,%rax,1)
   2865e:	00 00 
   28660:	4a 8d 3c 23          	lea    (%rbx,%r12,1),%rdi
   28664:	48 81 ff 7f 51 01 00 	cmp    $0x1517f,%rdi
   2866b:	0f 87 e4 00 00 00    	ja     28755 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0xd85>
   28671:	f2 0f 10 ac f4 a0 00 	movsd  0xa0(%rsp,%rsi,8),%xmm5
   28678:	00 00 
   2867a:	f2 0f 59 ec          	mulsd  %xmm4,%xmm5
   2867e:	f2 43 0f 10 34 e6    	movsd  (%r14,%r12,8),%xmm6
   28684:	f2 0f 59 f5          	mulsd  %xmm5,%xmm6
   28688:	f2 43 0f 58 34 e3    	addsd  (%r11,%r12,8),%xmm6
   2868e:	f2 43 0f 11 34 e3    	movsd  %xmm6,(%r11,%r12,8)
   28694:	4a 8d 3c 26          	lea    (%rsi,%r12,1),%rdi
   28698:	48 ff c7             	inc    %rdi
   2869b:	49 ff c4             	inc    %r12
   2869e:	48 83 ff 0a          	cmp    $0xa,%rdi
   286a2:	75 bc                	jne    28660 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0xc90>
   286a4:	4c 8d 24 76          	lea    (%rsi,%rsi,2),%r12
   286a8:	4d 01 d4             	add    %r10,%r12
   286ab:	48 8b 74 24 20       	mov    0x20(%rsp),%rsi
   286b0:	49 39 f4             	cmp    %rsi,%r12
   286b3:	0f 83 e9 0a 00 00    	jae    291a2 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x17d2>
   286b9:	48 8b 74 24 18       	mov    0x18(%rsp),%rsi
   286be:	66 0f 28 f0          	movapd %xmm0,%xmm6
   286c2:	f2 0f 59 f5          	mulsd  %xmm5,%xmm6
   286c6:	f2 42 0f 58 34 e6    	addsd  (%rsi,%r12,8),%xmm6
   286cc:	f2 42 0f 11 34 e6    	movsd  %xmm6,(%rsi,%r12,8)
   286d2:	49 8d 7c 24 01       	lea    0x1(%r12),%rdi
   286d7:	48 8b 74 24 20       	mov    0x20(%rsp),%rsi
   286dc:	48 39 f7             	cmp    %rsi,%rdi
   286df:	0f 83 c0 0a 00 00    	jae    291a5 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x17d5>
   286e5:	48 8b 74 24 18       	mov    0x18(%rsp),%rsi
   286ea:	66 0f 28 f1          	movapd %xmm1,%xmm6
   286ee:	f2 0f 59 f5          	mulsd  %xmm5,%xmm6
   286f2:	f2 42 0f 58 74 e6 08 	addsd  0x8(%rsi,%r12,8),%xmm6
   286f9:	f2 42 0f 11 74 e6 08 	movsd  %xmm6,0x8(%rsi,%r12,8)
   28700:	49 8d 7c 24 02       	lea    0x2(%r12),%rdi
   28705:	48 8b 74 24 20       	mov    0x20(%rsp),%rsi
   2870a:	48 39 f7             	cmp    %rsi,%rdi
   2870d:	0f 83 92 0a 00 00    	jae    291a5 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x17d5>
   28713:	48 8b 74 24 18       	mov    0x18(%rsp),%rsi
   28718:	f2 0f 59 ea          	mulsd  %xmm2,%xmm5
   2871c:	f2 42 0f 58 6c e6 10 	addsd  0x10(%rsi,%r12,8),%xmm5
   28723:	f2 42 0f 11 6c e6 10 	movsd  %xmm5,0x10(%rsi,%r12,8)
   2872a:	49 83 c6 08          	add    $0x8,%r14
   2872e:	48 83 c3 0b          	add    $0xb,%rbx
   28732:	49 83 c3 58          	add    $0x58,%r11
   28736:	4c 89 fe             	mov    %r15,%rsi
   28739:	49 83 ff 0a          	cmp    $0xa,%r15
   2873d:	4c 8d ac 24 f0 00 00 	lea    0xf0(%rsp),%r13
   28744:	00 
   28745:	48 8b 6c 24 08       	mov    0x8(%rsp),%rbp
   2874a:	0f 85 00 ff ff ff    	jne    28650 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0xc80>
   28750:	e9 9b fe ff ff       	jmp    285f0 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0xc20>
   28755:	48 8d 15 bc 8a 04 00 	lea    0x48abc(%rip),%rdx        # 71218 <anon.22e4ac0f336c447f4ab0ae585f38cc41.18.llvm.10198508844383412456+0x258>
   2875c:	bb 80 51 01 00       	mov    $0x15180,%ebx
   28761:	48 89 de             	mov    %rbx,%rsi
   28764:	ff 15 ae b0 04 00    	call   *0x4b0ae(%rip)        # 73818 <_DYNAMIC+0x328>
   2876a:	e9 4b 12 00 00       	jmp    299ba <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1fea>
   2876f:	48 8d bc 24 f0 00 00 	lea    0xf0(%rsp),%rdi
   28776:	00 
   28777:	be 40 65 00 00       	mov    $0x6540,%esi
   2877c:	e8 af df ff ff       	call   26730 <<f64 as alloc::vec::spec_from_elem::SpecFromElem>::from_elem::<alloc::alloc::Global>>
   28781:	b8 80 51 01 00       	mov    $0x15180,%eax
   28786:	31 c9                	xor    %ecx,%ecx
   28788:	31 d2                	xor    %edx,%edx
   2878a:	48 8b 74 24 78       	mov    0x78(%rsp),%rsi
   2878f:	31 ff                	xor    %edi,%edi
   28791:	4c 8d 04 ff          	lea    (%rdi,%rdi,8),%r8
   28795:	4c 89 84 24 e0 01 00 	mov    %r8,0x1e0(%rsp)
   2879c:	00 
   2879d:	48 ff c7             	inc    %rdi
   287a0:	48 89 bc 24 28 01 00 	mov    %rdi,0x128(%rsp)
   287a7:	00 
   287a8:	48 89 8c 24 58 01 00 	mov    %rcx,0x158(%rsp)
   287af:	00 
   287b0:	48 89 94 24 50 01 00 	mov    %rdx,0x150(%rsp)
   287b7:	00 
   287b8:	48 89 b4 24 48 01 00 	mov    %rsi,0x148(%rsp)
   287bf:	00 
   287c0:	48 89 84 24 d0 01 00 	mov    %rax,0x1d0(%rsp)
   287c7:	00 
   287c8:	31 ff                	xor    %edi,%edi
   287ca:	eb 4d                	jmp    28819 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0xe49>
   287cc:	48 8b 84 24 90 00 00 	mov    0x90(%rsp),%rax
   287d3:	00 
   287d4:	48 05 e0 fc ff ff    	add    $0xfffffffffffffce0,%rax
   287da:	48 8b b4 24 60 01 00 	mov    0x160(%rsp),%rsi
   287e1:	00 
   287e2:	48 81 c6 00 19 00 00 	add    $0x1900,%rsi
   287e9:	48 8b 94 24 30 01 00 	mov    0x130(%rsp),%rdx
   287f0:	00 
   287f1:	48 81 c2 20 03 00 00 	add    $0x320,%rdx
   287f8:	48 8b 8c 24 70 01 00 	mov    0x170(%rsp),%rcx
   287ff:	00 
   28800:	48 81 c1 f0 00 00 00 	add    $0xf0,%rcx
   28807:	48 8b bc 24 f0 01 00 	mov    0x1f0(%rsp),%rdi
   2880e:	00 
   2880f:	48 83 ff 09          	cmp    $0x9,%rdi
   28813:	0f 84 88 08 00 00    	je     290a1 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x16d1>
   28819:	49 89 f8             	mov    %rdi,%r8
   2881c:	48 ff c7             	inc    %rdi
   2881f:	48 89 bc 24 f0 01 00 	mov    %rdi,0x1f0(%rsp)
   28826:	00 
   28827:	4c 03 84 24 e0 01 00 	add    0x1e0(%rsp),%r8
   2882e:	00 
   2882f:	49 c1 e0 03          	shl    $0x3,%r8
   28833:	4c 89 84 24 80 01 00 	mov    %r8,0x180(%rsp)
   2883a:	00 
   2883b:	48 89 8c 24 70 01 00 	mov    %rcx,0x170(%rsp)
   28842:	00 
   28843:	49 89 cc             	mov    %rcx,%r12
   28846:	48 89 94 24 30 01 00 	mov    %rdx,0x130(%rsp)
   2884d:	00 
   2884e:	48 89 d3             	mov    %rdx,%rbx
   28851:	48 89 b4 24 60 01 00 	mov    %rsi,0x160(%rsp)
   28858:	00 
   28859:	48 89 74 24 50       	mov    %rsi,0x50(%rsp)
   2885e:	48 89 84 24 90 00 00 	mov    %rax,0x90(%rsp)
   28865:	00 
   28866:	48 89 44 24 60       	mov    %rax,0x60(%rsp)
   2886b:	31 ed                	xor    %ebp,%ebp
   2886d:	eb 41                	jmp    288b0 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0xee0>
   2886f:	90                   	nop
   28870:	be 20 03 00 00       	mov    $0x320,%esi
   28875:	ba 08 00 00 00       	mov    $0x8,%edx
   2887a:	4c 89 f7             	mov    %r14,%rdi
   2887d:	ff d3                	call   *%rbx
   2887f:	48 83 44 24 60 9c    	addq   $0xffffffffffffff9c,0x60(%rsp)
   28885:	48 81 44 24 50 20 03 	addq   $0x320,0x50(%rsp)
   2888c:	00 00 
   2888e:	48 8b 9c 24 10 01 00 	mov    0x110(%rsp),%rbx
   28895:	00 
   28896:	48 83 c3 64          	add    $0x64,%rbx
   2889a:	49 83 c4 1e          	add    $0x1e,%r12
   2889e:	48 8b 44 24 70       	mov    0x70(%rsp),%rax
   288a3:	48 89 c5             	mov    %rax,%rbp
   288a6:	48 83 f8 08          	cmp    $0x8,%rax
   288aa:	0f 84 1c ff ff ff    	je     287cc <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0xdfc>
   288b0:	ff 15 c2 ae 04 00    	call   *0x4aec2(%rip)        # 73778 <_DYNAMIC+0x288>
   288b6:	41 bf 20 03 00 00    	mov    $0x320,%r15d
   288bc:	41 bd 08 00 00 00    	mov    $0x8,%r13d
   288c2:	bf 20 03 00 00       	mov    $0x320,%edi
   288c7:	be 08 00 00 00       	mov    $0x8,%esi
   288cc:	ff 15 e6 af 04 00    	call   *0x4afe6(%rip)        # 738b8 <_DYNAMIC+0x3c8>
   288d2:	48 85 c0             	test   %rax,%rax
   288d5:	0f 84 b1 10 00 00    	je     2998c <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1fbc>
   288db:	49 89 c6             	mov    %rax,%r14
   288de:	48 8d 45 01          	lea    0x1(%rbp),%rax
   288e2:	48 89 44 24 70       	mov    %rax,0x70(%rsp)
   288e7:	48 03 ac 24 80 01 00 	add    0x180(%rsp),%rbp
   288ee:	00 
   288ef:	48 89 ac 24 80 00 00 	mov    %rbp,0x80(%rsp)
   288f6:	00 
   288f7:	49 8d 46 18          	lea    0x18(%r14),%rax
   288fb:	b9 64 00 00 00       	mov    $0x64,%ecx
   28900:	48 89 9c 24 10 01 00 	mov    %rbx,0x110(%rsp)
   28907:	00 
   28908:	48 89 da             	mov    %rbx,%rdx
   2890b:	48 8b 74 24 60       	mov    0x60(%rsp),%rsi
   28910:	4d 89 f0             	mov    %r14,%r8
   28913:	4c 8b 4c 24 50       	mov    0x50(%rsp),%r9
   28918:	45 31 d2             	xor    %r10d,%r10d
   2891b:	4c 89 e3             	mov    %r12,%rbx
   2891e:	4c 89 74 24 08       	mov    %r14,0x8(%rsp)
   28923:	66 66 66 66 2e 0f 1f 	data16 data16 data16 cs nopw 0x0(%rax,%rax,1)
   2892a:	84 00 00 00 00 00 
   28930:	4d 85 d2             	test   %r10,%r10
   28933:	74 1b                	je     28950 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0xf80>
   28935:	4b 8d 3c 12          	lea    (%r10,%r10,1),%rdi
   28939:	48 8d 3c bf          	lea    (%rdi,%rdi,4),%rdi
   2893d:	49 83 fa 04          	cmp    $0x4,%r10
   28941:	73 1d                	jae    28960 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0xf90>
   28943:	45 31 db             	xor    %r11d,%r11d
   28946:	eb 77                	jmp    289bf <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0xfef>
   28948:	0f 1f 84 00 00 00 00 	nopl   0x0(%rax,%rax,1)
   2894f:	00 
   28950:	31 ff                	xor    %edi,%edi
   28952:	e9 a1 00 00 00       	jmp    289f8 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1028>
   28957:	66 0f 1f 84 00 00 00 	nopw   0x0(%rax,%rax,1)
   2895e:	00 00 
   28960:	4d 89 d7             	mov    %r10,%r15
   28963:	49 83 e7 fc          	and    $0xfffffffffffffffc,%r15
   28967:	4d 89 cd             	mov    %r9,%r13
   2896a:	45 31 db             	xor    %r11d,%r11d
   2896d:	0f 1f 00             	nopl   (%rax)
   28970:	f2 41 0f 10 45 00    	movsd  0x0(%r13),%xmm0
   28976:	f2 42 0f 11 44 d8 e8 	movsd  %xmm0,-0x18(%rax,%r11,8)
   2897d:	f2 41 0f 10 45 50    	movsd  0x50(%r13),%xmm0
   28983:	f2 42 0f 11 44 d8 f0 	movsd  %xmm0,-0x10(%rax,%r11,8)
   2898a:	f2 41 0f 10 85 a0 00 	movsd  0xa0(%r13),%xmm0
   28991:	00 00 
   28993:	f2 42 0f 11 44 d8 f8 	movsd  %xmm0,-0x8(%rax,%r11,8)
   2899a:	f2 41 0f 10 85 f0 00 	movsd  0xf0(%r13),%xmm0
   289a1:	00 00 
   289a3:	f2 42 0f 11 04 d8    	movsd  %xmm0,(%rax,%r11,8)
   289a9:	49 83 c3 04          	add    $0x4,%r11
   289ad:	49 81 c5 40 01 00 00 	add    $0x140,%r13
   289b4:	4d 39 df             	cmp    %r11,%r15
   289b7:	75 b7                	jne    28970 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0xfa0>
   289b9:	41 f6 c2 03          	test   $0x3,%r10b
   289bd:	74 39                	je     289f8 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1028>
   289bf:	45 89 d7             	mov    %r10d,%r15d
   289c2:	41 83 e7 03          	and    $0x3,%r15d
   289c6:	4f 8d 2c d8          	lea    (%r8,%r11,8),%r13
   289ca:	4f 8d 1c 9b          	lea    (%r11,%r11,4),%r11
   289ce:	49 c1 e3 04          	shl    $0x4,%r11
   289d2:	4d 01 cb             	add    %r9,%r11
   289d5:	45 31 e4             	xor    %r12d,%r12d
   289d8:	0f 1f 84 00 00 00 00 	nopl   0x0(%rax,%rax,1)
   289df:	00 
   289e0:	f2 41 0f 10 03       	movsd  (%r11),%xmm0
   289e5:	f2 43 0f 11 44 e5 00 	movsd  %xmm0,0x0(%r13,%r12,8)
   289ec:	49 ff c4             	inc    %r12
   289ef:	49 83 c3 50          	add    $0x50,%r11
   289f3:	4d 39 e7             	cmp    %r12,%r15
   289f6:	75 e8                	jne    289e0 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1010>
   289f8:	4d 8d 5a 01          	lea    0x1(%r10),%r11
   289fc:	4e 8d 3c 17          	lea    (%rdi,%r10,1),%r15
   28a00:	4f 8d 2c fe          	lea    (%r14,%r15,8),%r13
   28a04:	48 01 d7             	add    %rdx,%rdi
   28a07:	4c 8b 74 24 78       	mov    0x78(%rsp),%r14
   28a0c:	4d 8d 24 fe          	lea    (%r14,%rdi,8),%r12
   28a10:	31 ed                	xor    %ebp,%ebp
   28a12:	66 66 66 66 66 2e 0f 	data16 data16 data16 data16 cs nopw 0x0(%rax,%rax,1)
   28a19:	1f 84 00 00 00 00 00 
   28a20:	48 39 ee             	cmp    %rbp,%rsi
   28a23:	0f 84 37 07 00 00    	je     29160 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1790>
   28a29:	48 39 e9             	cmp    %rbp,%rcx
   28a2c:	0f 84 3f 07 00 00    	je     29171 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x17a1>
   28a32:	f2 41 0f 10 04 ec    	movsd  (%r12,%rbp,8),%xmm0
   28a38:	f2 41 0f 11 44 ed 00 	movsd  %xmm0,0x0(%r13,%rbp,8)
   28a3f:	4d 8d 34 2a          	lea    (%r10,%rbp,1),%r14
   28a43:	49 ff c6             	inc    %r14
   28a46:	48 ff c5             	inc    %rbp
   28a49:	49 83 fe 0a          	cmp    $0xa,%r14
   28a4d:	75 d1                	jne    28a20 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1050>
   28a4f:	49 83 c1 08          	add    $0x8,%r9
   28a53:	48 83 c0 50          	add    $0x50,%rax
   28a57:	49 83 c0 50          	add    $0x50,%r8
   28a5b:	48 83 c1 f5          	add    $0xfffffffffffffff5,%rcx
   28a5f:	48 83 c6 f5          	add    $0xfffffffffffffff5,%rsi
   28a63:	48 ff c2             	inc    %rdx
   28a66:	4d 89 da             	mov    %r11,%r10
   28a69:	49 83 fb 0a          	cmp    $0xa,%r11
   28a6d:	4c 8b 74 24 08       	mov    0x8(%rsp),%r14
   28a72:	0f 85 b8 fe ff ff    	jne    28930 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0xf60>
   28a78:	49 89 dc             	mov    %rbx,%r12
   28a7b:	f2 45 0f 10 16       	movsd  (%r14),%xmm10
   28a80:	f2 41 0f 10 7e 58    	movsd  0x58(%r14),%xmm7
   28a86:	66 41 0f 28 c2       	movapd %xmm10,%xmm0
   28a8b:	f2 0f 58 c7          	addsd  %xmm7,%xmm0
   28a8f:	f2 45 0f 10 86 b0 00 	movsd  0xb0(%r14),%xmm8
   28a96:	00 00 
   28a98:	f2 41 0f 58 c0       	addsd  %xmm8,%xmm0
   28a9d:	f2 45 0f 10 8e 08 01 	movsd  0x108(%r14),%xmm9
   28aa4:	00 00 
   28aa6:	f2 41 0f 58 c1       	addsd  %xmm9,%xmm0
   28aab:	f2 41 0f 10 8e 60 01 	movsd  0x160(%r14),%xmm1
   28ab2:	00 00 
   28ab4:	f2 0f 58 c1          	addsd  %xmm1,%xmm0
   28ab8:	f2 41 0f 10 96 b8 01 	movsd  0x1b8(%r14),%xmm2
   28abf:	00 00 
   28ac1:	f2 0f 58 c2          	addsd  %xmm2,%xmm0
   28ac5:	f2 41 0f 10 9e 10 02 	movsd  0x210(%r14),%xmm3
   28acc:	00 00 
   28ace:	f2 0f 58 c3          	addsd  %xmm3,%xmm0
   28ad2:	f2 41 0f 10 a6 68 02 	movsd  0x268(%r14),%xmm4
   28ad9:	00 00 
   28adb:	f2 0f 58 c4          	addsd  %xmm4,%xmm0
   28adf:	f2 41 0f 10 ae c0 02 	movsd  0x2c0(%r14),%xmm5
   28ae6:	00 00 
   28ae8:	f2 0f 58 c5          	addsd  %xmm5,%xmm0
   28aec:	f2 41 0f 10 b6 18 03 	movsd  0x318(%r14),%xmm6
   28af3:	00 00 
   28af5:	f2 0f 58 c6          	addsd  %xmm6,%xmm0
   28af9:	f2 44 0f 10 1d 3e 42 	movsd  -0x1bdc2(%rip),%xmm11        # cd40 <GCC_except_table0+0xb8>
   28b00:	fe ff 
   28b02:	f2 41 0f 5e c3       	divsd  %xmm11,%xmm0
   28b07:	f2 0f 59 05 69 42 fe 	mulsd  -0x1bd97(%rip),%xmm0        # cd78 <GCC_except_table0+0xf0>
   28b0e:	ff 
   28b0f:	f2 0f 5f 05 41 42 fe 	maxsd  -0x1bdbf(%rip),%xmm0        # cd58 <GCC_except_table0+0xd0>
   28b16:	ff 
   28b17:	f2 44 0f 58 d0       	addsd  %xmm0,%xmm10
   28b1c:	f2 45 0f 11 16       	movsd  %xmm10,(%r14)
   28b21:	f2 0f 58 f8          	addsd  %xmm0,%xmm7
   28b25:	f2 41 0f 11 7e 58    	movsd  %xmm7,0x58(%r14)
   28b2b:	f2 44 0f 58 c0       	addsd  %xmm0,%xmm8
   28b30:	f2 45 0f 11 86 b0 00 	movsd  %xmm8,0xb0(%r14)
   28b37:	00 00 
   28b39:	f2 44 0f 58 c8       	addsd  %xmm0,%xmm9
   28b3e:	f2 45 0f 11 8e 08 01 	movsd  %xmm9,0x108(%r14)
   28b45:	00 00 
   28b47:	f2 41 0f 59 c3       	mulsd  %xmm11,%xmm0
   28b4c:	f2 0f 58 c8          	addsd  %xmm0,%xmm1
   28b50:	f2 41 0f 11 8e 60 01 	movsd  %xmm1,0x160(%r14)
   28b57:	00 00 
   28b59:	f2 0f 58 d0          	addsd  %xmm0,%xmm2
   28b5d:	f2 41 0f 11 96 b8 01 	movsd  %xmm2,0x1b8(%r14)
   28b64:	00 00 
   28b66:	f2 0f 58 d8          	addsd  %xmm0,%xmm3
   28b6a:	f2 41 0f 11 9e 10 02 	movsd  %xmm3,0x210(%r14)
   28b71:	00 00 
   28b73:	f2 0f 58 e0          	addsd  %xmm0,%xmm4
   28b77:	f2 41 0f 11 a6 68 02 	movsd  %xmm4,0x268(%r14)
   28b7e:	00 00 
   28b80:	f2 0f 58 e8          	addsd  %xmm0,%xmm5
   28b84:	f2 41 0f 11 ae c0 02 	movsd  %xmm5,0x2c0(%r14)
   28b8b:	00 00 
   28b8d:	f2 0f 58 c6          	addsd  %xmm6,%xmm0
   28b91:	f2 41 0f 11 86 18 03 	movsd  %xmm0,0x318(%r14)
   28b98:	00 00 
   28b9a:	48 8b 9c 24 80 00 00 	mov    0x80(%rsp),%rbx
   28ba1:	00 
   28ba2:	48 8d 04 1b          	lea    (%rbx,%rbx,1),%rax
   28ba6:	48 c1 e3 05          	shl    $0x5,%rbx
   28baa:	48 29 c3             	sub    %rax,%rbx
   28bad:	48 c7 44 24 48 00 00 	movq   $0x0,0x48(%rsp)
   28bb4:	00 00 
   28bb6:	48 89 9c 24 98 00 00 	mov    %rbx,0x98(%rsp)
   28bbd:	00 
   28bbe:	48 c7 84 24 b8 00 00 	movq   $0x0,0xb8(%rsp)
   28bc5:	00 00 00 00 00 
   28bca:	48 c7 84 24 c0 00 00 	movq   $0xa,0xc0(%rsp)
   28bd1:	00 0a 00 00 00 
   28bd6:	48 8d 44 24 10       	lea    0x10(%rsp),%rax
   28bdb:	48 89 84 24 a0 00 00 	mov    %rax,0xa0(%rsp)
   28be2:	00 
   28be3:	48 8d 84 24 98 00 00 	lea    0x98(%rsp),%rax
   28bea:	00 
   28beb:	48 89 84 24 a8 00 00 	mov    %rax,0xa8(%rsp)
   28bf2:	00 
   28bf3:	48 8d 44 24 48       	lea    0x48(%rsp),%rax
   28bf8:	48 89 84 24 b0 00 00 	mov    %rax,0xb0(%rsp)
   28bff:	00 
   28c00:	48 8d 7c 24 28       	lea    0x28(%rsp),%rdi
   28c05:	4c 8d bc 24 a0 00 00 	lea    0xa0(%rsp),%r15
   28c0c:	00 
   28c0d:	4c 89 fe             	mov    %r15,%rsi
   28c10:	e8 5b 10 00 00       	call   29c70 <<alloc::vec::Vec<f64> as alloc::vec::spec_from_iter_nested::SpecFromIterNested<f64, core::iter::adapters::map::Map<core::ops::range::Range<usize>, xdremux_engine::apple_photographic_styles::apple_style_revert_local_data::{closure#3}>>>::from_iter>
   28c15:	48 8b 6c 24 30       	mov    0x30(%rsp),%rbp
   28c1a:	4c 8b 44 24 38       	mov    0x38(%rsp),%r8
   28c1f:	ba 64 00 00 00       	mov    $0x64,%edx
   28c24:	4c 89 ff             	mov    %r15,%rdi
   28c27:	4c 89 f6             	mov    %r14,%rsi
   28c2a:	48 89 e9             	mov    %rbp,%rcx
   28c2d:	e8 9e dd ff ff       	call   269d0 <xdremux_engine::apple_photographic_styles::solve_linear_system>
   28c32:	80 bc 24 a0 00 00 00 	cmpb   $0xff,0xa0(%rsp)
   28c39:	ff 
   28c3a:	74 34                	je     28c70 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x12a0>
   28c3c:	ff 15 36 ab 04 00    	call   *0x4ab36(%rip)        # 73778 <_DYNAMIC+0x288>
   28c42:	bf 50 00 00 00       	mov    $0x50,%edi
   28c47:	be 08 00 00 00       	mov    $0x8,%esi
   28c4c:	ff 15 66 ac 04 00    	call   *0x4ac66(%rip)        # 738b8 <_DYNAMIC+0x3c8>
   28c52:	48 85 c0             	test   %rax,%rax
   28c55:	0f 84 1f 0d 00 00    	je     2997a <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1faa>
   28c5b:	49 89 c5             	mov    %rax,%r13
   28c5e:	48 83 c0 50          	add    $0x50,%rax
   28c62:	41 bf 0a 00 00 00    	mov    $0xa,%r15d
   28c68:	eb 2e                	jmp    28c98 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x12c8>
   28c6a:	66 0f 1f 44 00 00    	nopw   0x0(%rax,%rax,1)
   28c70:	4c 8b bc 24 a8 00 00 	mov    0xa8(%rsp),%r15
   28c77:	00 
   28c78:	4c 8b ac 24 b0 00 00 	mov    0xb0(%rsp),%r13
   28c7f:	00 
   28c80:	48 8b 84 24 b8 00 00 	mov    0xb8(%rsp),%rax
   28c87:	00 
   28c88:	48 85 c0             	test   %rax,%rax
   28c8b:	74 7f                	je     28d0c <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x133c>
   28c8d:	48 8d 04 c5 00 00 00 	lea    0x0(,%rax,8),%rax
   28c94:	00 
   28c95:	4c 01 e8             	add    %r13,%rax
   28c98:	4c 8d 1d 71 47 fe ff 	lea    -0x1b88f(%rip),%r11        # d410 <GCC_except_table0+0x788>
   28c9f:	66 0f 28 15 d9 46 fe 	movapd -0x1b927(%rip),%xmm2        # d380 <GCC_except_table0+0x6f8>
   28ca6:	ff 
   28ca7:	48 8b 8c 24 f8 00 00 	mov    0xf8(%rsp),%rcx
   28cae:	00 
   28caf:	48 8b b4 24 00 01 00 	mov    0x100(%rsp),%rsi
   28cb6:	00 
   28cb7:	31 d2                	xor    %edx,%edx
   28cb9:	4d 89 e8             	mov    %r13,%r8
   28cbc:	0f 1f 40 00          	nopl   0x0(%rax)
   28cc0:	4c 8b 4c 24 48       	mov    0x48(%rsp),%r9
   28cc5:	49 8d 3c 14          	lea    (%r12,%rdx,1),%rdi
   28cc9:	4c 01 cf             	add    %r9,%rdi
   28ccc:	48 39 f7             	cmp    %rsi,%rdi
   28ccf:	0f 83 32 0b 00 00    	jae    29807 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1e37>
   28cd5:	49 01 d1             	add    %rdx,%r9
   28cd8:	45 31 d2             	xor    %r10d,%r10d
   28cdb:	49 83 f9 0c          	cmp    $0xc,%r9
   28cdf:	41 0f 93 c2          	setae  %r10b
   28ce3:	f2 43 0f 10 04 d3    	movsd  (%r11,%r10,8),%xmm0
   28ce9:	66 0f 28 c8          	movapd %xmm0,%xmm1
   28ced:	66 0f 57 ca          	xorpd  %xmm2,%xmm1
   28cf1:	f2 41 0f 5f 08       	maxsd  (%r8),%xmm1
   28cf6:	49 83 c0 08          	add    $0x8,%r8
   28cfa:	f2 0f 5d c1          	minsd  %xmm1,%xmm0
   28cfe:	f2 0f 11 04 f9       	movsd  %xmm0,(%rcx,%rdi,8)
   28d03:	48 83 c2 03          	add    $0x3,%rdx
   28d07:	49 39 c0             	cmp    %rax,%r8
   28d0a:	75 b4                	jne    28cc0 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x12f0>
   28d0c:	4d 85 ff             	test   %r15,%r15
   28d0f:	74 15                	je     28d26 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1356>
   28d11:	49 c1 e7 03          	shl    $0x3,%r15
   28d15:	ba 08 00 00 00       	mov    $0x8,%edx
   28d1a:	4c 89 ef             	mov    %r13,%rdi
   28d1d:	4c 89 fe             	mov    %r15,%rsi
   28d20:	ff 15 1a aa 04 00    	call   *0x4aa1a(%rip)        # 73740 <_DYNAMIC+0x250>
   28d26:	48 8b 74 24 28       	mov    0x28(%rsp),%rsi
   28d2b:	48 85 f6             	test   %rsi,%rsi
   28d2e:	74 12                	je     28d42 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1372>
   28d30:	48 c1 e6 03          	shl    $0x3,%rsi
   28d34:	ba 08 00 00 00       	mov    $0x8,%edx
   28d39:	48 89 ef             	mov    %rbp,%rdi
   28d3c:	ff 15 fe a9 04 00    	call   *0x4a9fe(%rip)        # 73740 <_DYNAMIC+0x250>
   28d42:	48 c7 44 24 48 01 00 	movq   $0x1,0x48(%rsp)
   28d49:	00 00 
   28d4b:	48 89 9c 24 98 00 00 	mov    %rbx,0x98(%rsp)
   28d52:	00 
   28d53:	48 c7 84 24 b8 00 00 	movq   $0x0,0xb8(%rsp)
   28d5a:	00 00 00 00 00 
   28d5f:	48 c7 84 24 c0 00 00 	movq   $0xa,0xc0(%rsp)
   28d66:	00 0a 00 00 00 
   28d6b:	48 8d 44 24 10       	lea    0x10(%rsp),%rax
   28d70:	48 89 84 24 a0 00 00 	mov    %rax,0xa0(%rsp)
   28d77:	00 
   28d78:	48 8d 84 24 98 00 00 	lea    0x98(%rsp),%rax
   28d7f:	00 
   28d80:	48 89 84 24 a8 00 00 	mov    %rax,0xa8(%rsp)
   28d87:	00 
   28d88:	48 8d 44 24 48       	lea    0x48(%rsp),%rax
   28d8d:	48 89 84 24 b0 00 00 	mov    %rax,0xb0(%rsp)
   28d94:	00 
   28d95:	48 8d 7c 24 28       	lea    0x28(%rsp),%rdi
   28d9a:	4c 8d bc 24 a0 00 00 	lea    0xa0(%rsp),%r15
   28da1:	00 
   28da2:	4c 89 fe             	mov    %r15,%rsi
   28da5:	e8 c6 0e 00 00       	call   29c70 <<alloc::vec::Vec<f64> as alloc::vec::spec_from_iter_nested::SpecFromIterNested<f64, core::iter::adapters::map::Map<core::ops::range::Range<usize>, xdremux_engine::apple_photographic_styles::apple_style_revert_local_data::{closure#3}>>>::from_iter>
   28daa:	48 8b 6c 24 30       	mov    0x30(%rsp),%rbp
   28daf:	4c 8b 44 24 38       	mov    0x38(%rsp),%r8
   28db4:	ba 64 00 00 00       	mov    $0x64,%edx
   28db9:	4c 89 ff             	mov    %r15,%rdi
   28dbc:	4c 89 f6             	mov    %r14,%rsi
   28dbf:	48 89 e9             	mov    %rbp,%rcx
   28dc2:	e8 09 dc ff ff       	call   269d0 <xdremux_engine::apple_photographic_styles::solve_linear_system>
   28dc7:	80 bc 24 a0 00 00 00 	cmpb   $0xff,0xa0(%rsp)
   28dce:	ff 
   28dcf:	74 2f                	je     28e00 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1430>
   28dd1:	ff 15 a1 a9 04 00    	call   *0x4a9a1(%rip)        # 73778 <_DYNAMIC+0x288>
   28dd7:	bf 50 00 00 00       	mov    $0x50,%edi
   28ddc:	be 08 00 00 00       	mov    $0x8,%esi
   28de1:	ff 15 d1 aa 04 00    	call   *0x4aad1(%rip)        # 738b8 <_DYNAMIC+0x3c8>
   28de7:	48 85 c0             	test   %rax,%rax
   28dea:	0f 84 8a 0b 00 00    	je     2997a <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1faa>
   28df0:	49 89 c5             	mov    %rax,%r13
   28df3:	48 83 c0 50          	add    $0x50,%rax
   28df7:	41 bf 0a 00 00 00    	mov    $0xa,%r15d
   28dfd:	eb 29                	jmp    28e28 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1458>
   28dff:	90                   	nop
   28e00:	4c 8b bc 24 a8 00 00 	mov    0xa8(%rsp),%r15
   28e07:	00 
   28e08:	4c 8b ac 24 b0 00 00 	mov    0xb0(%rsp),%r13
   28e0f:	00 
   28e10:	48 8b 84 24 b8 00 00 	mov    0xb8(%rsp),%rax
   28e17:	00 
   28e18:	48 85 c0             	test   %rax,%rax
   28e1b:	74 7f                	je     28e9c <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x14cc>
   28e1d:	48 8d 04 c5 00 00 00 	lea    0x0(,%rax,8),%rax
   28e24:	00 
   28e25:	4c 01 e8             	add    %r13,%rax
   28e28:	4c 8d 1d e1 45 fe ff 	lea    -0x1ba1f(%rip),%r11        # d410 <GCC_except_table0+0x788>
   28e2f:	66 0f 28 15 49 45 fe 	movapd -0x1bab7(%rip),%xmm2        # d380 <GCC_except_table0+0x6f8>
   28e36:	ff 
   28e37:	48 8b 8c 24 f8 00 00 	mov    0xf8(%rsp),%rcx
   28e3e:	00 
   28e3f:	48 8b b4 24 00 01 00 	mov    0x100(%rsp),%rsi
   28e46:	00 
   28e47:	31 d2                	xor    %edx,%edx
   28e49:	4d 89 e8             	mov    %r13,%r8
   28e4c:	0f 1f 40 00          	nopl   0x0(%rax)
   28e50:	4c 8b 4c 24 48       	mov    0x48(%rsp),%r9
   28e55:	49 8d 3c 14          	lea    (%r12,%rdx,1),%rdi
   28e59:	4c 01 cf             	add    %r9,%rdi
   28e5c:	48 39 f7             	cmp    %rsi,%rdi
   28e5f:	0f 83 a2 09 00 00    	jae    29807 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1e37>
   28e65:	49 01 d1             	add    %rdx,%r9
   28e68:	45 31 d2             	xor    %r10d,%r10d
   28e6b:	49 83 f9 0c          	cmp    $0xc,%r9
   28e6f:	41 0f 93 c2          	setae  %r10b
   28e73:	f2 43 0f 10 04 d3    	movsd  (%r11,%r10,8),%xmm0
   28e79:	66 0f 28 c8          	movapd %xmm0,%xmm1
   28e7d:	66 0f 57 ca          	xorpd  %xmm2,%xmm1
   28e81:	f2 41 0f 5f 08       	maxsd  (%r8),%xmm1
   28e86:	49 83 c0 08          	add    $0x8,%r8
   28e8a:	f2 0f 5d c1          	minsd  %xmm1,%xmm0
   28e8e:	f2 0f 11 04 f9       	movsd  %xmm0,(%rcx,%rdi,8)
   28e93:	48 83 c2 03          	add    $0x3,%rdx
   28e97:	49 39 c0             	cmp    %rax,%r8
   28e9a:	75 b4                	jne    28e50 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1480>
   28e9c:	4d 85 ff             	test   %r15,%r15
   28e9f:	74 15                	je     28eb6 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x14e6>
   28ea1:	49 c1 e7 03          	shl    $0x3,%r15
   28ea5:	ba 08 00 00 00       	mov    $0x8,%edx
   28eaa:	4c 89 ef             	mov    %r13,%rdi
   28ead:	4c 89 fe             	mov    %r15,%rsi
   28eb0:	ff 15 8a a8 04 00    	call   *0x4a88a(%rip)        # 73740 <_DYNAMIC+0x250>
   28eb6:	48 8b 74 24 28       	mov    0x28(%rsp),%rsi
   28ebb:	48 85 f6             	test   %rsi,%rsi
   28ebe:	74 12                	je     28ed2 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1502>
   28ec0:	48 c1 e6 03          	shl    $0x3,%rsi
   28ec4:	ba 08 00 00 00       	mov    $0x8,%edx
   28ec9:	48 89 ef             	mov    %rbp,%rdi
   28ecc:	ff 15 6e a8 04 00    	call   *0x4a86e(%rip)        # 73740 <_DYNAMIC+0x250>
   28ed2:	48 c7 44 24 48 02 00 	movq   $0x2,0x48(%rsp)
   28ed9:	00 00 
   28edb:	48 89 9c 24 98 00 00 	mov    %rbx,0x98(%rsp)
   28ee2:	00 
   28ee3:	48 c7 84 24 b8 00 00 	movq   $0x0,0xb8(%rsp)
   28eea:	00 00 00 00 00 
   28eef:	48 c7 84 24 c0 00 00 	movq   $0xa,0xc0(%rsp)
   28ef6:	00 0a 00 00 00 
   28efb:	48 8d 44 24 10       	lea    0x10(%rsp),%rax
   28f00:	48 89 84 24 a0 00 00 	mov    %rax,0xa0(%rsp)
   28f07:	00 
   28f08:	48 8d 84 24 98 00 00 	lea    0x98(%rsp),%rax
   28f0f:	00 
   28f10:	48 89 84 24 a8 00 00 	mov    %rax,0xa8(%rsp)
   28f17:	00 
   28f18:	48 8d 44 24 48       	lea    0x48(%rsp),%rax
   28f1d:	48 89 84 24 b0 00 00 	mov    %rax,0xb0(%rsp)
   28f24:	00 
   28f25:	48 8d 7c 24 28       	lea    0x28(%rsp),%rdi
   28f2a:	4c 8d bc 24 a0 00 00 	lea    0xa0(%rsp),%r15
   28f31:	00 
   28f32:	4c 89 fe             	mov    %r15,%rsi
   28f35:	e8 36 0d 00 00       	call   29c70 <<alloc::vec::Vec<f64> as alloc::vec::spec_from_iter_nested::SpecFromIterNested<f64, core::iter::adapters::map::Map<core::ops::range::Range<usize>, xdremux_engine::apple_photographic_styles::apple_style_revert_local_data::{closure#3}>>>::from_iter>
   28f3a:	48 8b 6c 24 30       	mov    0x30(%rsp),%rbp
   28f3f:	4c 8b 44 24 38       	mov    0x38(%rsp),%r8
   28f44:	ba 64 00 00 00       	mov    $0x64,%edx
   28f49:	4c 89 ff             	mov    %r15,%rdi
   28f4c:	4c 89 f6             	mov    %r14,%rsi
   28f4f:	48 89 e9             	mov    %rbp,%rcx
   28f52:	e8 79 da ff ff       	call   269d0 <xdremux_engine::apple_photographic_styles::solve_linear_system>
   28f57:	80 bc 24 a0 00 00 00 	cmpb   $0xff,0xa0(%rsp)
   28f5e:	ff 
   28f5f:	74 4f                	je     28fb0 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x15e0>
   28f61:	ff 15 11 a8 04 00    	call   *0x4a811(%rip)        # 73778 <_DYNAMIC+0x288>
   28f67:	bf 50 00 00 00       	mov    $0x50,%edi
   28f6c:	be 08 00 00 00       	mov    $0x8,%esi
   28f71:	ff 15 41 a9 04 00    	call   *0x4a941(%rip)        # 738b8 <_DYNAMIC+0x3c8>
   28f77:	48 85 c0             	test   %rax,%rax
   28f7a:	0f 84 fa 09 00 00    	je     2997a <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1faa>
   28f80:	49 89 c5             	mov    %rax,%r13
   28f83:	48 83 c0 50          	add    $0x50,%rax
   28f87:	41 bf 0a 00 00 00    	mov    $0xa,%r15d
   28f8d:	48 8b 1d ac a7 04 00 	mov    0x4a7ac(%rip),%rbx        # 73740 <_DYNAMIC+0x250>
   28f94:	4c 8d 1d 75 44 fe ff 	lea    -0x1bb8b(%rip),%r11        # d410 <GCC_except_table0+0x788>
   28f9b:	66 0f 28 15 dd 43 fe 	movapd -0x1bc23(%rip),%xmm2        # d380 <GCC_except_table0+0x6f8>
   28fa2:	ff 
   28fa3:	eb 4d                	jmp    28ff2 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1622>
   28fa5:	66 66 2e 0f 1f 84 00 	data16 cs nopw 0x0(%rax,%rax,1)
   28fac:	00 00 00 00 
   28fb0:	4c 8b bc 24 a8 00 00 	mov    0xa8(%rsp),%r15
   28fb7:	00 
   28fb8:	4c 8b ac 24 b0 00 00 	mov    0xb0(%rsp),%r13
   28fbf:	00 
   28fc0:	48 8b 84 24 b8 00 00 	mov    0xb8(%rsp),%rax
   28fc7:	00 
   28fc8:	48 85 c0             	test   %rax,%rax
   28fcb:	4c 8d 1d 3e 44 fe ff 	lea    -0x1bbc2(%rip),%r11        # d410 <GCC_except_table0+0x788>
   28fd2:	66 0f 28 15 a6 43 fe 	movapd -0x1bc5a(%rip),%xmm2        # d380 <GCC_except_table0+0x6f8>
   28fd9:	ff 
   28fda:	0f 84 b3 00 00 00    	je     29093 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x16c3>
   28fe0:	48 8d 04 c5 00 00 00 	lea    0x0(,%rax,8),%rax
   28fe7:	00 
   28fe8:	4c 01 e8             	add    %r13,%rax
   28feb:	48 8b 1d 4e a7 04 00 	mov    0x4a74e(%rip),%rbx        # 73740 <_DYNAMIC+0x250>
   28ff2:	48 8b 8c 24 f8 00 00 	mov    0xf8(%rsp),%rcx
   28ff9:	00 
   28ffa:	48 8b b4 24 00 01 00 	mov    0x100(%rsp),%rsi
   29001:	00 
   29002:	31 d2                	xor    %edx,%edx
   29004:	4d 89 e8             	mov    %r13,%r8
   29007:	66 0f 1f 84 00 00 00 	nopw   0x0(%rax,%rax,1)
   2900e:	00 00 
   29010:	4c 8b 4c 24 48       	mov    0x48(%rsp),%r9
   29015:	49 8d 3c 14          	lea    (%r12,%rdx,1),%rdi
   29019:	4c 01 cf             	add    %r9,%rdi
   2901c:	48 39 f7             	cmp    %rsi,%rdi
   2901f:	0f 83 e2 07 00 00    	jae    29807 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1e37>
   29025:	49 01 d1             	add    %rdx,%r9
   29028:	45 31 d2             	xor    %r10d,%r10d
   2902b:	49 83 f9 0c          	cmp    $0xc,%r9
   2902f:	41 0f 93 c2          	setae  %r10b
   29033:	f2 43 0f 10 04 d3    	movsd  (%r11,%r10,8),%xmm0
   29039:	66 0f 28 c8          	movapd %xmm0,%xmm1
   2903d:	66 0f 57 ca          	xorpd  %xmm2,%xmm1
   29041:	f2 41 0f 5f 08       	maxsd  (%r8),%xmm1
   29046:	49 83 c0 08          	add    $0x8,%r8
   2904a:	f2 0f 5d c1          	minsd  %xmm1,%xmm0
   2904e:	f2 0f 11 04 f9       	movsd  %xmm0,(%rcx,%rdi,8)
   29053:	48 83 c2 03          	add    $0x3,%rdx
   29057:	49 39 c0             	cmp    %rax,%r8
   2905a:	75 b4                	jne    29010 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1640>
   2905c:	4d 85 ff             	test   %r15,%r15
   2905f:	74 11                	je     29072 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x16a2>
   29061:	49 c1 e7 03          	shl    $0x3,%r15
   29065:	ba 08 00 00 00       	mov    $0x8,%edx
   2906a:	4c 89 ef             	mov    %r13,%rdi
   2906d:	4c 89 fe             	mov    %r15,%rsi
   29070:	ff d3                	call   *%rbx
   29072:	48 8b 74 24 28       	mov    0x28(%rsp),%rsi
   29077:	48 85 f6             	test   %rsi,%rsi
   2907a:	0f 84 f0 f7 ff ff    	je     28870 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0xea0>
   29080:	48 c1 e6 03          	shl    $0x3,%rsi
   29084:	ba 08 00 00 00       	mov    $0x8,%edx
   29089:	48 89 ef             	mov    %rbp,%rdi
   2908c:	ff d3                	call   *%rbx
   2908e:	e9 dd f7 ff ff       	jmp    28870 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0xea0>
   29093:	48 8b 1d a6 a6 04 00 	mov    0x4a6a6(%rip),%rbx        # 73740 <_DYNAMIC+0x250>
   2909a:	4d 85 ff             	test   %r15,%r15
   2909d:	75 c2                	jne    29061 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1691>
   2909f:	eb d1                	jmp    29072 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x16a2>
   290a1:	48 8b 84 24 d0 01 00 	mov    0x1d0(%rsp),%rax
   290a8:	00 
   290a9:	48 05 e0 e3 ff ff    	add    $0xffffffffffffe3e0,%rax
   290af:	48 8b b4 24 48 01 00 	mov    0x148(%rsp),%rsi
   290b6:	00 
   290b7:	48 81 c6 00 e1 00 00 	add    $0xe100,%rsi
   290be:	48 8b 94 24 50 01 00 	mov    0x150(%rsp),%rdx
   290c5:	00 
   290c6:	48 81 c2 20 1c 00 00 	add    $0x1c20,%rdx
   290cd:	48 8b 8c 24 58 01 00 	mov    0x158(%rsp),%rcx
   290d4:	00 
   290d5:	48 81 c1 70 08 00 00 	add    $0x870,%rcx
   290dc:	48 8b bc 24 28 01 00 	mov    0x128(%rsp),%rdi
   290e3:	00 
   290e4:	48 83 ff 0c          	cmp    $0xc,%rdi
   290e8:	0f 85 a3 f6 ff ff    	jne    28791 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0xdc1>
   290ee:	ff 15 84 a6 04 00    	call   *0x4a684(%rip)        # 73778 <_DYNAMIC+0x288>
   290f4:	41 bf 80 ca 00 00    	mov    $0xca80,%r15d
   290fa:	41 bd 01 00 00 00    	mov    $0x1,%r13d
   29100:	bf 80 ca 00 00       	mov    $0xca80,%edi
   29105:	be 01 00 00 00       	mov    $0x1,%esi
   2910a:	ff 15 70 a6 04 00    	call   *0x4a670(%rip)        # 73780 <_DYNAMIC+0x290>
   29110:	48 85 c0             	test   %rax,%rax
   29113:	0f 84 73 08 00 00    	je     2998c <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1fbc>
   29119:	48 89 c5             	mov    %rax,%rbp
   2911c:	48 c7 44 24 28 80 ca 	movq   $0xca80,0x28(%rsp)
   29123:	00 00 
   29125:	48 89 44 24 30       	mov    %rax,0x30(%rsp)
   2912a:	48 c7 44 24 38 00 00 	movq   $0x0,0x38(%rsp)
   29131:	00 00 
   29133:	48 8b 84 24 f8 00 00 	mov    0xf8(%rsp),%rax
   2913a:	00 
   2913b:	48 8b 8c 24 00 01 00 	mov    0x100(%rsp),%rcx
   29142:	00 
   29143:	48 89 4c 24 50       	mov    %rcx,0x50(%rsp)
   29148:	48 89 84 24 60 01 00 	mov    %rax,0x160(%rsp)
   2914f:	00 
   29150:	4c 8d 60 10          	lea    0x10(%rax),%r12
   29154:	31 db                	xor    %ebx,%ebx
   29156:	45 31 ed             	xor    %r13d,%r13d
   29159:	31 d2                	xor    %edx,%edx
   2915b:	e9 87 00 00 00       	jmp    291e7 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1817>
   29160:	48 01 ef             	add    %rbp,%rdi
   29163:	48 8d 15 1e 80 04 00 	lea    0x4801e(%rip),%rdx        # 71188 <anon.22e4ac0f336c447f4ab0ae585f38cc41.18.llvm.10198508844383412456+0x1c8>
   2916a:	be 80 51 01 00       	mov    $0x15180,%esi
   2916f:	eb 12                	jmp    29183 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x17b3>
   29171:	49 01 ef             	add    %rbp,%r15
   29174:	48 8d 15 25 80 04 00 	lea    0x48025(%rip),%rdx        # 711a0 <anon.22e4ac0f336c447f4ab0ae585f38cc41.18.llvm.10198508844383412456+0x1e0>
   2917b:	be 64 00 00 00       	mov    $0x64,%esi
   29180:	4c 89 ff             	mov    %r15,%rdi
   29183:	4c 8b 74 24 08       	mov    0x8(%rsp),%r14
   29188:	ff 15 8a a6 04 00    	call   *0x4a68a(%rip)        # 73818 <_DYNAMIC+0x328>
   2918e:	e9 27 08 00 00       	jmp    299ba <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1fea>
   29193:	4c 89 c7             	mov    %r8,%rdi
   29196:	eb 0d                	jmp    291a5 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x17d5>
   29198:	4c 89 ef             	mov    %r13,%rdi
   2919b:	eb 08                	jmp    291a5 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x17d5>
   2919d:	48 89 ef             	mov    %rbp,%rdi
   291a0:	eb 03                	jmp    291a5 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x17d5>
   291a2:	4c 89 e7             	mov    %r12,%rdi
   291a5:	48 8d 15 54 80 04 00 	lea    0x48054(%rip),%rdx        # 71200 <anon.22e4ac0f336c447f4ab0ae585f38cc41.18.llvm.10198508844383412456+0x240>
   291ac:	ff 15 66 a6 04 00    	call   *0x4a666(%rip)        # 73818 <_DYNAMIC+0x328>
   291b2:	e9 03 08 00 00       	jmp    299ba <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1fea>
   291b7:	4c 8b a4 24 80 01 00 	mov    0x180(%rsp),%r12
   291be:	00 
   291bf:	49 81 c4 80 43 00 00 	add    $0x4380,%r12
   291c6:	48 8b 9c 24 70 01 00 	mov    0x170(%rsp),%rbx
   291cd:	00 
   291ce:	48 81 c3 70 08 00 00 	add    $0x870,%rbx
   291d5:	48 8b 94 24 30 01 00 	mov    0x130(%rsp),%rdx
   291dc:	00 
   291dd:	48 83 fa 0c          	cmp    $0xc,%rdx
   291e1:	0f 84 49 06 00 00    	je     29830 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1e60>
   291e7:	48 ff c2             	inc    %rdx
   291ea:	48 89 94 24 30 01 00 	mov    %rdx,0x130(%rsp)
   291f1:	00 
   291f2:	48 89 9c 24 70 01 00 	mov    %rbx,0x170(%rsp)
   291f9:	00 
   291fa:	4c 89 a4 24 80 01 00 	mov    %r12,0x180(%rsp)
   29201:	00 
   29202:	31 c0                	xor    %eax,%eax
   29204:	eb 29                	jmp    2922f <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x185f>
   29206:	4c 8b a4 24 10 01 00 	mov    0x110(%rsp),%r12
   2920d:	00 
   2920e:	49 81 c4 80 07 00 00 	add    $0x780,%r12
   29215:	48 8b 9c 24 80 00 00 	mov    0x80(%rsp),%rbx
   2921c:	00 
   2921d:	48 81 c3 f0 00 00 00 	add    $0xf0,%rbx
   29224:	48 8b 44 24 70       	mov    0x70(%rsp),%rax
   29229:	48 83 f8 09          	cmp    $0x9,%rax
   2922d:	74 88                	je     291b7 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x17e7>
   2922f:	48 ff c0             	inc    %rax
   29232:	48 89 44 24 70       	mov    %rax,0x70(%rsp)
   29237:	48 89 9c 24 80 00 00 	mov    %rbx,0x80(%rsp)
   2923e:	00 
   2923f:	4c 89 a4 24 10 01 00 	mov    %r12,0x110(%rsp)
   29246:	00 
   29247:	31 c0                	xor    %eax,%eax
   29249:	eb 1b                	jmp    29266 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1896>
   2924b:	0f 1f 44 00 00       	nopl   0x0(%rax,%rax,1)
   29250:	49 81 c4 f0 00 00 00 	add    $0xf0,%r12
   29257:	48 83 c3 1e          	add    $0x1e,%rbx
   2925b:	48 8b 44 24 60       	mov    0x60(%rsp),%rax
   29260:	48 83 f8 08          	cmp    $0x8,%rax
   29264:	74 a0                	je     29206 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1836>
   29266:	48 ff c0             	inc    %rax
   29269:	48 89 44 24 60       	mov    %rax,0x60(%rsp)
   2926e:	49 c7 c7 fd ff ff ff 	mov    $0xfffffffffffffffd,%r15
   29275:	66 66 2e 0f 1f 84 00 	data16 cs nopw 0x0(%rax,%rax,1)
   2927c:	00 00 00 00 
   29280:	66 0f 57 c0          	xorpd  %xmm0,%xmm0
   29284:	f3 0f 11 44 24 08    	movss  %xmm0,0x8(%rsp)
   2928a:	49 83 ff 08          	cmp    $0x8,%r15
   2928e:	77 13                	ja     292a3 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x18d3>
   29290:	48 8d 05 fd 6e fe ff 	lea    -0x19103(%rip),%rax        # 10194 <anon.22e4ac0f336c447f4ab0ae585f38cc41.16.llvm.10198508844383412456+0x87e>
   29297:	f3 42 0f 10 04 b8    	movss  (%rax,%r15,4),%xmm0
   2929d:	f3 0f 11 44 24 08    	movss  %xmm0,0x8(%rsp)
   292a3:	4a 8d 3c 3b          	lea    (%rbx,%r15,1),%rdi
   292a7:	48 83 c7 03          	add    $0x3,%rdi
   292ab:	48 3b 7c 24 50       	cmp    0x50(%rsp),%rdi
   292b0:	0f 83 63 05 00 00    	jae    29819 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1e49>
   292b6:	f2 43 0f 10 44 fc 08 	movsd  0x8(%r12,%r15,8),%xmm0
   292bd:	f2 0f 5a c0          	cvtsd2ss %xmm0,%xmm0
   292c1:	f3 0f 10 4c 24 08    	movss  0x8(%rsp),%xmm1
   292c7:	f3 0f 58 c8          	addss  %xmm0,%xmm1
   292cb:	f3 0f 11 4c 24 08    	movss  %xmm1,0x8(%rsp)
   292d1:	48 8b 05 f8 a7 04 00 	mov    0x4a7f8(%rip),%rax        # 73ad0 <_DYNAMIC+0x5e0>
   292d8:	48 8b 00             	mov    (%rax),%rax
   292db:	48 85 c0             	test   %rax,%rax
   292de:	0f 84 f8 00 00 00    	je     293dc <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1a0c>
   292e4:	48 b9 00 00 00 00 00 	movabs $0x40000000000000,%rcx
   292eb:	00 40 00 
   292ee:	48 85 c8             	test   %rcx,%rax
   292f1:	0f 85 f9 00 00 00    	jne    293f0 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1a20>
   292f7:	66 0f 6e 44 24 08    	movd   0x8(%rsp),%xmm0
   292fd:	66 0f 7e c1          	movd   %xmm0,%ecx
   29301:	89 c8                	mov    %ecx,%eax
   29303:	25 00 00 00 80       	and    $0x80000000,%eax
   29308:	89 ce                	mov    %ecx,%esi
   2930a:	81 e6 00 00 80 7f    	and    $0x7f800000,%esi
   29310:	89 ca                	mov    %ecx,%edx
   29312:	81 e2 ff ff 7f 00    	and    $0x7fffff,%edx
   29318:	81 fe 00 00 80 7f    	cmp    $0x7f800000,%esi
   2931e:	75 30                	jne    29350 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1980>
   29320:	31 c9                	xor    %ecx,%ecx
   29322:	85 d2                	test   %edx,%edx
   29324:	0f 95 c1             	setne  %cl
   29327:	c1 e1 09             	shl    $0x9,%ecx
   2932a:	c1 e8 10             	shr    $0x10,%eax
   2932d:	c1 ea 0d             	shr    $0xd,%edx
   29330:	09 c2                	or     %eax,%edx
   29332:	09 ca                	or     %ecx,%edx
   29334:	81 ca 00 7c 00 00    	or     $0x7c00,%edx
   2933a:	41 89 d6             	mov    %edx,%r14d
   2933d:	e9 bc 00 00 00       	jmp    293fe <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1a2e>
   29342:	66 66 66 66 66 2e 0f 	data16 data16 data16 data16 cs nopw 0x0(%rax,%rax,1)
   29349:	1f 84 00 00 00 00 00 
   29350:	c1 e8 10             	shr    $0x10,%eax
   29353:	81 fe 00 00 00 47    	cmp    $0x47000000,%esi
   29359:	76 0d                	jbe    29368 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1998>
   2935b:	0d 00 7c 00 00       	or     $0x7c00,%eax
   29360:	41 89 c6             	mov    %eax,%r14d
   29363:	e9 96 00 00 00       	jmp    293fe <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1a2e>
   29368:	81 fe 00 00 80 38    	cmp    $0x38800000,%esi
   2936e:	73 42                	jae    293b2 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x19e2>
   29370:	81 fe 00 00 00 33    	cmp    $0x33000000,%esi
   29376:	72 e8                	jb     29360 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1990>
   29378:	c1 ee 17             	shr    $0x17,%esi
   2937b:	81 ca 00 00 80 00    	or     $0x800000,%edx
   29381:	b1 7e                	mov    $0x7e,%cl
   29383:	40 28 f1             	sub    %sil,%cl
   29386:	41 89 d6             	mov    %edx,%r14d
   29389:	41 d3 ee             	shr    %cl,%r14d
   2938c:	b9 1d 00 00 00       	mov    $0x1d,%ecx
   29391:	29 f1                	sub    %esi,%ecx
   29393:	83 e1 1f             	and    $0x1f,%ecx
   29396:	0f a3 ca             	bt     %ecx,%edx
   29399:	73 12                	jae    293ad <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x19dd>
   2939b:	be 03 00 00 00       	mov    $0x3,%esi
   293a0:	d3 e6                	shl    %cl,%esi
   293a2:	ff ce                	dec    %esi
   293a4:	21 d6                	and    %edx,%esi
   293a6:	83 fe 01             	cmp    $0x1,%esi
   293a9:	41 83 de ff          	sbb    $0xffffffff,%r14d
   293ad:	41 09 c6             	or     %eax,%r14d
   293b0:	eb 4c                	jmp    293fe <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1a2e>
   293b2:	c1 ee 0d             	shr    $0xd,%esi
   293b5:	c1 ea 0d             	shr    $0xd,%edx
   293b8:	89 cf                	mov    %ecx,%edi
   293ba:	c1 ef 0c             	shr    $0xc,%edi
   293bd:	f7 c1 ff 2f 00 00    	test   $0x2fff,%ecx
   293c3:	0f 95 c1             	setne  %cl
   293c6:	40 20 f9             	and    %dil,%cl
   293c9:	01 f2                	add    %esi,%edx
   293cb:	81 c2 00 40 00 00    	add    $0x4000,%edx
   293d1:	09 c2                	or     %eax,%edx
   293d3:	44 0f b6 f1          	movzbl %cl,%r14d
   293d7:	41 01 d6             	add    %edx,%r14d
   293da:	eb 22                	jmp    293fe <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1a2e>
   293dc:	ff 15 f6 a6 04 00    	call   *0x4a6f6(%rip)        # 73ad8 <_DYNAMIC+0x5e8>
   293e2:	48 0f ba e0 36       	bt     $0x36,%rax
   293e7:	0f 83 0a ff ff ff    	jae    292f7 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1927>
   293ed:	0f 1f 00             	nopl   (%rax)
   293f0:	66 0f 6e 44 24 08    	movd   0x8(%rsp),%xmm0
   293f6:	e8 d5 06 00 00       	call   29ad0 <half::binary16::arch::x86::f32_to_f16_x86_f16c>
   293fb:	41 89 c6             	mov    %eax,%r14d
   293fe:	48 8b 44 24 28       	mov    0x28(%rsp),%rax
   29403:	4c 29 e8             	sub    %r13,%rax
   29406:	48 83 f8 01          	cmp    $0x1,%rax
   2940a:	0f 86 77 01 00 00    	jbe    29587 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1bb7>
   29410:	66 46 89 74 2d 00    	mov    %r14w,0x0(%rbp,%r13,1)
   29416:	49 83 c5 02          	add    $0x2,%r13
   2941a:	4c 89 6c 24 38       	mov    %r13,0x38(%rsp)
   2941f:	49 8d 47 01          	lea    0x1(%r15),%rax
   29423:	66 0f ef c0          	pxor   %xmm0,%xmm0
   29427:	66 0f 7e 44 24 08    	movd   %xmm0,0x8(%rsp)
   2942d:	48 83 f8 08          	cmp    $0x8,%rax
   29431:	77 14                	ja     29447 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1a77>
   29433:	48 8d 05 5a 6d fe ff 	lea    -0x192a6(%rip),%rax        # 10194 <anon.22e4ac0f336c447f4ab0ae585f38cc41.16.llvm.10198508844383412456+0x87e>
   2943a:	66 42 0f 6e 44 b8 04 	movd   0x4(%rax,%r15,4),%xmm0
   29441:	66 0f 7e 44 24 08    	movd   %xmm0,0x8(%rsp)
   29447:	4a 8d 3c 3b          	lea    (%rbx,%r15,1),%rdi
   2944b:	48 83 c7 04          	add    $0x4,%rdi
   2944f:	48 3b 7c 24 50       	cmp    0x50(%rsp),%rdi
   29454:	0f 83 bf 03 00 00    	jae    29819 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1e49>
   2945a:	f2 43 0f 10 44 fc 10 	movsd  0x10(%r12,%r15,8),%xmm0
   29461:	f2 0f 5a c0          	cvtsd2ss %xmm0,%xmm0
   29465:	f3 0f 10 4c 24 08    	movss  0x8(%rsp),%xmm1
   2946b:	f3 0f 58 c8          	addss  %xmm0,%xmm1
   2946f:	f3 0f 11 4c 24 08    	movss  %xmm1,0x8(%rsp)
   29475:	48 8b 05 54 a6 04 00 	mov    0x4a654(%rip),%rax        # 73ad0 <_DYNAMIC+0x5e0>
   2947c:	48 8b 00             	mov    (%rax),%rax
   2947f:	48 85 c0             	test   %rax,%rax
   29482:	0f 84 2b 01 00 00    	je     295b3 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1be3>
   29488:	48 b9 00 00 00 00 00 	movabs $0x40000000000000,%rcx
   2948f:	00 40 00 
   29492:	48 85 c8             	test   %rcx,%rax
   29495:	0f 85 35 01 00 00    	jne    295d0 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1c00>
   2949b:	66 0f 6e 44 24 08    	movd   0x8(%rsp),%xmm0
   294a1:	66 0f 7e c1          	movd   %xmm0,%ecx
   294a5:	89 c8                	mov    %ecx,%eax
   294a7:	25 00 00 00 80       	and    $0x80000000,%eax
   294ac:	89 ce                	mov    %ecx,%esi
   294ae:	81 e6 00 00 80 7f    	and    $0x7f800000,%esi
   294b4:	89 ca                	mov    %ecx,%edx
   294b6:	81 e2 ff ff 7f 00    	and    $0x7fffff,%edx
   294bc:	81 fe 00 00 80 7f    	cmp    $0x7f800000,%esi
   294c2:	75 2c                	jne    294f0 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1b20>
   294c4:	31 c9                	xor    %ecx,%ecx
   294c6:	85 d2                	test   %edx,%edx
   294c8:	0f 95 c1             	setne  %cl
   294cb:	c1 e1 09             	shl    $0x9,%ecx
   294ce:	c1 e8 10             	shr    $0x10,%eax
   294d1:	c1 ea 0d             	shr    $0xd,%edx
   294d4:	09 c2                	or     %eax,%edx
   294d6:	09 ca                	or     %ecx,%edx
   294d8:	81 ca 00 7c 00 00    	or     $0x7c00,%edx
   294de:	41 89 d6             	mov    %edx,%r14d
   294e1:	e9 f8 00 00 00       	jmp    295de <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1c0e>
   294e6:	66 2e 0f 1f 84 00 00 	cs nopw 0x0(%rax,%rax,1)
   294ed:	00 00 00 
   294f0:	c1 e8 10             	shr    $0x10,%eax
   294f3:	81 fe 00 00 00 47    	cmp    $0x47000000,%esi
   294f9:	76 0d                	jbe    29508 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1b38>
   294fb:	0d 00 7c 00 00       	or     $0x7c00,%eax
   29500:	41 89 c6             	mov    %eax,%r14d
   29503:	e9 d6 00 00 00       	jmp    295de <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1c0e>
   29508:	81 fe 00 00 80 38    	cmp    $0x38800000,%esi
   2950e:	73 10                	jae    29520 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1b50>
   29510:	81 fe 00 00 00 33    	cmp    $0x33000000,%esi
   29516:	73 35                	jae    2954d <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1b7d>
   29518:	41 89 c6             	mov    %eax,%r14d
   2951b:	e9 be 00 00 00       	jmp    295de <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1c0e>
   29520:	c1 ee 0d             	shr    $0xd,%esi
   29523:	c1 ea 0d             	shr    $0xd,%edx
   29526:	89 cf                	mov    %ecx,%edi
   29528:	c1 ef 0c             	shr    $0xc,%edi
   2952b:	f7 c1 ff 2f 00 00    	test   $0x2fff,%ecx
   29531:	0f 95 c1             	setne  %cl
   29534:	40 20 f9             	and    %dil,%cl
   29537:	01 f2                	add    %esi,%edx
   29539:	81 c2 00 40 00 00    	add    $0x4000,%edx
   2953f:	09 c2                	or     %eax,%edx
   29541:	44 0f b6 f1          	movzbl %cl,%r14d
   29545:	41 01 d6             	add    %edx,%r14d
   29548:	e9 91 00 00 00       	jmp    295de <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1c0e>
   2954d:	c1 ee 17             	shr    $0x17,%esi
   29550:	81 ca 00 00 80 00    	or     $0x800000,%edx
   29556:	b1 7e                	mov    $0x7e,%cl
   29558:	40 28 f1             	sub    %sil,%cl
   2955b:	41 89 d6             	mov    %edx,%r14d
   2955e:	41 d3 ee             	shr    %cl,%r14d
   29561:	b9 1d 00 00 00       	mov    $0x1d,%ecx
   29566:	29 f1                	sub    %esi,%ecx
   29568:	83 e1 1f             	and    $0x1f,%ecx
   2956b:	0f a3 ca             	bt     %ecx,%edx
   2956e:	73 12                	jae    29582 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1bb2>
   29570:	be 03 00 00 00       	mov    $0x3,%esi
   29575:	d3 e6                	shl    %cl,%esi
   29577:	ff ce                	dec    %esi
   29579:	21 d6                	and    %edx,%esi
   2957b:	83 fe 01             	cmp    $0x1,%esi
   2957e:	41 83 de ff          	sbb    $0xffffffff,%r14d
   29582:	41 09 c6             	or     %eax,%r14d
   29585:	eb 57                	jmp    295de <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1c0e>
   29587:	ba 02 00 00 00       	mov    $0x2,%edx
   2958c:	b9 01 00 00 00       	mov    $0x1,%ecx
   29591:	41 b8 01 00 00 00    	mov    $0x1,%r8d
   29597:	48 8d 7c 24 28       	lea    0x28(%rsp),%rdi
   2959c:	4c 89 ee             	mov    %r13,%rsi
   2959f:	e8 1c 08 00 00       	call   29dc0 <<alloc::raw_vec::RawVecInner<_>>::reserve::do_reserve_and_handle::<alloc::alloc::Global>>
   295a4:	48 8b 6c 24 30       	mov    0x30(%rsp),%rbp
   295a9:	4c 8b 6c 24 38       	mov    0x38(%rsp),%r13
   295ae:	e9 5d fe ff ff       	jmp    29410 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1a40>
   295b3:	ff 15 1f a5 04 00    	call   *0x4a51f(%rip)        # 73ad8 <_DYNAMIC+0x5e8>
   295b9:	48 0f ba e0 36       	bt     $0x36,%rax
   295be:	0f 83 d7 fe ff ff    	jae    2949b <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1acb>
   295c4:	66 66 66 2e 0f 1f 84 	data16 data16 cs nopw 0x0(%rax,%rax,1)
   295cb:	00 00 00 00 00 
   295d0:	66 0f 6e 44 24 08    	movd   0x8(%rsp),%xmm0
   295d6:	e8 f5 04 00 00       	call   29ad0 <half::binary16::arch::x86::f32_to_f16_x86_f16c>
   295db:	41 89 c6             	mov    %eax,%r14d
   295de:	48 8b 44 24 28       	mov    0x28(%rsp),%rax
   295e3:	4c 29 e8             	sub    %r13,%rax
   295e6:	48 83 f8 01          	cmp    $0x1,%rax
   295ea:	0f 86 77 01 00 00    	jbe    29767 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1d97>
   295f0:	48 8b 6c 24 30       	mov    0x30(%rsp),%rbp
   295f5:	66 46 89 74 2d 00    	mov    %r14w,0x0(%rbp,%r13,1)
   295fb:	49 83 c5 02          	add    $0x2,%r13
   295ff:	4c 89 6c 24 38       	mov    %r13,0x38(%rsp)
   29604:	49 8d 47 02          	lea    0x2(%r15),%rax
   29608:	66 0f ef c0          	pxor   %xmm0,%xmm0
   2960c:	66 0f 7e 44 24 08    	movd   %xmm0,0x8(%rsp)
   29612:	48 83 f8 08          	cmp    $0x8,%rax
   29616:	77 14                	ja     2962c <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1c5c>
   29618:	48 8d 05 75 6b fe ff 	lea    -0x1948b(%rip),%rax        # 10194 <anon.22e4ac0f336c447f4ab0ae585f38cc41.16.llvm.10198508844383412456+0x87e>
   2961f:	66 42 0f 6e 44 b8 08 	movd   0x8(%rax,%r15,4),%xmm0
   29626:	66 0f 7e 44 24 08    	movd   %xmm0,0x8(%rsp)
   2962c:	4a 8d 3c 3b          	lea    (%rbx,%r15,1),%rdi
   29630:	48 83 c7 05          	add    $0x5,%rdi
   29634:	48 3b 7c 24 50       	cmp    0x50(%rsp),%rdi
   29639:	0f 83 da 01 00 00    	jae    29819 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1e49>
   2963f:	f2 43 0f 10 44 fc 18 	movsd  0x18(%r12,%r15,8),%xmm0
   29646:	f2 0f 5a c0          	cvtsd2ss %xmm0,%xmm0
   2964a:	f3 0f 10 4c 24 08    	movss  0x8(%rsp),%xmm1
   29650:	f3 0f 58 c8          	addss  %xmm0,%xmm1
   29654:	f3 0f 11 4c 24 08    	movss  %xmm1,0x8(%rsp)
   2965a:	48 8b 05 6f a4 04 00 	mov    0x4a46f(%rip),%rax        # 73ad0 <_DYNAMIC+0x5e0>
   29661:	48 8b 00             	mov    (%rax),%rax
   29664:	48 85 c0             	test   %rax,%rax
   29667:	0f 84 21 01 00 00    	je     2978e <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1dbe>
   2966d:	48 b9 00 00 00 00 00 	movabs $0x40000000000000,%rcx
   29674:	00 40 00 
   29677:	48 85 c8             	test   %rcx,%rax
   2967a:	0f 85 20 01 00 00    	jne    297a0 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1dd0>
   29680:	66 0f 6e 44 24 08    	movd   0x8(%rsp),%xmm0
   29686:	66 0f 7e c1          	movd   %xmm0,%ecx
   2968a:	89 c8                	mov    %ecx,%eax
   2968c:	25 00 00 00 80       	and    $0x80000000,%eax
   29691:	89 ce                	mov    %ecx,%esi
   29693:	81 e6 00 00 80 7f    	and    $0x7f800000,%esi
   29699:	89 ca                	mov    %ecx,%edx
   2969b:	81 e2 ff ff 7f 00    	and    $0x7fffff,%edx
   296a1:	81 fe 00 00 80 7f    	cmp    $0x7f800000,%esi
   296a7:	75 27                	jne    296d0 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1d00>
   296a9:	31 c9                	xor    %ecx,%ecx
   296ab:	85 d2                	test   %edx,%edx
   296ad:	0f 95 c1             	setne  %cl
   296b0:	c1 e1 09             	shl    $0x9,%ecx
   296b3:	c1 e8 10             	shr    $0x10,%eax
   296b6:	c1 ea 0d             	shr    $0xd,%edx
   296b9:	09 c2                	or     %eax,%edx
   296bb:	09 ca                	or     %ecx,%edx
   296bd:	81 ca 00 7c 00 00    	or     $0x7c00,%edx
   296c3:	41 89 d6             	mov    %edx,%r14d
   296c6:	e9 e3 00 00 00       	jmp    297ae <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1dde>
   296cb:	0f 1f 44 00 00       	nopl   0x0(%rax,%rax,1)
   296d0:	c1 e8 10             	shr    $0x10,%eax
   296d3:	81 fe 00 00 00 47    	cmp    $0x47000000,%esi
   296d9:	76 0d                	jbe    296e8 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1d18>
   296db:	0d 00 7c 00 00       	or     $0x7c00,%eax
   296e0:	41 89 c6             	mov    %eax,%r14d
   296e3:	e9 c6 00 00 00       	jmp    297ae <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1dde>
   296e8:	81 fe 00 00 80 38    	cmp    $0x38800000,%esi
   296ee:	73 10                	jae    29700 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1d30>
   296f0:	81 fe 00 00 00 33    	cmp    $0x33000000,%esi
   296f6:	73 35                	jae    2972d <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1d5d>
   296f8:	41 89 c6             	mov    %eax,%r14d
   296fb:	e9 ae 00 00 00       	jmp    297ae <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1dde>
   29700:	c1 ee 0d             	shr    $0xd,%esi
   29703:	c1 ea 0d             	shr    $0xd,%edx
   29706:	89 cf                	mov    %ecx,%edi
   29708:	c1 ef 0c             	shr    $0xc,%edi
   2970b:	f7 c1 ff 2f 00 00    	test   $0x2fff,%ecx
   29711:	0f 95 c1             	setne  %cl
   29714:	40 20 f9             	and    %dil,%cl
   29717:	01 f2                	add    %esi,%edx
   29719:	81 c2 00 40 00 00    	add    $0x4000,%edx
   2971f:	09 c2                	or     %eax,%edx
   29721:	44 0f b6 f1          	movzbl %cl,%r14d
   29725:	41 01 d6             	add    %edx,%r14d
   29728:	e9 81 00 00 00       	jmp    297ae <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1dde>
   2972d:	c1 ee 17             	shr    $0x17,%esi
   29730:	81 ca 00 00 80 00    	or     $0x800000,%edx
   29736:	b1 7e                	mov    $0x7e,%cl
   29738:	40 28 f1             	sub    %sil,%cl
   2973b:	41 89 d6             	mov    %edx,%r14d
   2973e:	41 d3 ee             	shr    %cl,%r14d
   29741:	b9 1d 00 00 00       	mov    $0x1d,%ecx
   29746:	29 f1                	sub    %esi,%ecx
   29748:	83 e1 1f             	and    $0x1f,%ecx
   2974b:	0f a3 ca             	bt     %ecx,%edx
   2974e:	73 12                	jae    29762 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1d92>
   29750:	be 03 00 00 00       	mov    $0x3,%esi
   29755:	d3 e6                	shl    %cl,%esi
   29757:	ff ce                	dec    %esi
   29759:	21 d6                	and    %edx,%esi
   2975b:	83 fe 01             	cmp    $0x1,%esi
   2975e:	41 83 de ff          	sbb    $0xffffffff,%r14d
   29762:	41 09 c6             	or     %eax,%r14d
   29765:	eb 47                	jmp    297ae <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1dde>
   29767:	ba 02 00 00 00       	mov    $0x2,%edx
   2976c:	b9 01 00 00 00       	mov    $0x1,%ecx
   29771:	41 b8 01 00 00 00    	mov    $0x1,%r8d
   29777:	48 8d 7c 24 28       	lea    0x28(%rsp),%rdi
   2977c:	4c 89 ee             	mov    %r13,%rsi
   2977f:	e8 3c 06 00 00       	call   29dc0 <<alloc::raw_vec::RawVecInner<_>>::reserve::do_reserve_and_handle::<alloc::alloc::Global>>
   29784:	4c 8b 6c 24 38       	mov    0x38(%rsp),%r13
   29789:	e9 62 fe ff ff       	jmp    295f0 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1c20>
   2978e:	ff 15 44 a3 04 00    	call   *0x4a344(%rip)        # 73ad8 <_DYNAMIC+0x5e8>
   29794:	48 0f ba e0 36       	bt     $0x36,%rax
   29799:	0f 83 e1 fe ff ff    	jae    29680 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1cb0>
   2979f:	90                   	nop
   297a0:	66 0f 6e 44 24 08    	movd   0x8(%rsp),%xmm0
   297a6:	e8 25 03 00 00       	call   29ad0 <half::binary16::arch::x86::f32_to_f16_x86_f16c>
   297ab:	41 89 c6             	mov    %eax,%r14d
   297ae:	48 8b 44 24 28       	mov    0x28(%rsp),%rax
   297b3:	4c 29 e8             	sub    %r13,%rax
   297b6:	48 83 f8 01          	cmp    $0x1,%rax
   297ba:	76 22                	jbe    297de <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1e0e>
   297bc:	66 46 89 74 2d 00    	mov    %r14w,0x0(%rbp,%r13,1)
   297c2:	49 83 c5 02          	add    $0x2,%r13
   297c6:	4c 89 6c 24 38       	mov    %r13,0x38(%rsp)
   297cb:	49 83 c7 03          	add    $0x3,%r15
   297cf:	49 83 ff 1b          	cmp    $0x1b,%r15
   297d3:	0f 85 a7 fa ff ff    	jne    29280 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x18b0>
   297d9:	e9 72 fa ff ff       	jmp    29250 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1880>
   297de:	ba 02 00 00 00       	mov    $0x2,%edx
   297e3:	b9 01 00 00 00       	mov    $0x1,%ecx
   297e8:	41 b8 01 00 00 00    	mov    $0x1,%r8d
   297ee:	48 8d 7c 24 28       	lea    0x28(%rsp),%rdi
   297f3:	4c 89 ee             	mov    %r13,%rsi
   297f6:	e8 c5 05 00 00       	call   29dc0 <<alloc::raw_vec::RawVecInner<_>>::reserve::do_reserve_and_handle::<alloc::alloc::Global>>
   297fb:	48 8b 6c 24 30       	mov    0x30(%rsp),%rbp
   29800:	4c 8b 6c 24 38       	mov    0x38(%rsp),%r13
   29805:	eb b5                	jmp    297bc <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1dec>
   29807:	48 8d 15 62 79 04 00 	lea    0x47962(%rip),%rdx        # 71170 <anon.22e4ac0f336c447f4ab0ae585f38cc41.18.llvm.10198508844383412456+0x1b0>
   2980e:	ff 15 04 a0 04 00    	call   *0x4a004(%rip)        # 73818 <_DYNAMIC+0x328>
   29814:	e9 a1 01 00 00       	jmp    299ba <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1fea>
   29819:	48 8d 15 38 79 04 00 	lea    0x47938(%rip),%rdx        # 71158 <anon.22e4ac0f336c447f4ab0ae585f38cc41.18.llvm.10198508844383412456+0x198>
   29820:	48 8b 74 24 50       	mov    0x50(%rsp),%rsi
   29825:	ff 15 ed 9f 04 00    	call   *0x49fed(%rip)        # 73818 <_DYNAMIC+0x328>
   2982b:	e9 8a 01 00 00       	jmp    299ba <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1fea>
   29830:	48 8d bc 24 a0 00 00 	lea    0xa0(%rsp),%rdi
   29837:	00 
   29838:	48 89 ee             	mov    %rbp,%rsi
   2983b:	4c 89 ea             	mov    %r13,%rdx
   2983e:	ff 15 b4 a2 04 00    	call   *0x4a2b4(%rip)        # 73af8 <_DYNAMIC+0x608>
   29844:	83 bc 24 a0 00 00 00 	cmpl   $0x1,0xa0(%rsp)
   2984b:	01 
   2984c:	75 79                	jne    298c7 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1ef7>
   2984e:	48 8b 84 24 a8 00 00 	mov    0xa8(%rsp),%rax
   29855:	00 
   29856:	f2 0f 10 84 24 b0 00 	movsd  0xb0(%rsp),%xmm0
   2985d:	00 00 
   2985f:	f3 0f 10 8c 24 c0 00 	movss  0xc0(%rsp),%xmm1
   29866:	00 00 
   29868:	0f b6 8c 24 c4 00 00 	movzbl 0xc4(%rsp),%ecx
   2986f:	00 
   29870:	0f b6 94 24 c7 00 00 	movzbl 0xc7(%rsp),%edx
   29877:	00 
   29878:	48 8b b4 24 20 01 00 	mov    0x120(%rsp),%rsi
   2987f:	00 
   29880:	88 56 1f             	mov    %dl,0x1f(%rsi)
   29883:	0f b7 94 24 c5 00 00 	movzwl 0xc5(%rsp),%edx
   2988a:	00 
   2988b:	66 89 56 1d          	mov    %dx,0x1d(%rsi)
   2988f:	f2 0f 10 94 24 b8 00 	movsd  0xb8(%rsp),%xmm2
   29896:	00 00 
   29898:	48 89 06             	mov    %rax,(%rsi)
   2989b:	f2 0f 11 46 08       	movsd  %xmm0,0x8(%rsi)
   298a0:	f2 0f 11 56 10       	movsd  %xmm2,0x10(%rsi)
   298a5:	f3 0f 11 4e 18       	movss  %xmm1,0x18(%rsi)
   298aa:	88 4e 1c             	mov    %cl,0x1c(%rsi)
   298ad:	48 8b 74 24 28       	mov    0x28(%rsp),%rsi
   298b2:	48 85 f6             	test   %rsi,%rsi
   298b5:	74 2d                	je     298e4 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1f14>
   298b7:	ba 01 00 00 00       	mov    $0x1,%edx
   298bc:	48 89 ef             	mov    %rbp,%rdi
   298bf:	ff 15 7b 9e 04 00    	call   *0x49e7b(%rip)        # 73740 <_DYNAMIC+0x250>
   298c5:	eb 1d                	jmp    298e4 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1f14>
   298c7:	48 8b 44 24 38       	mov    0x38(%rsp),%rax
   298cc:	48 8b 8c 24 20 01 00 	mov    0x120(%rsp),%rcx
   298d3:	00 
   298d4:	48 89 41 18          	mov    %rax,0x18(%rcx)
   298d8:	0f 10 44 24 28       	movups 0x28(%rsp),%xmm0
   298dd:	0f 11 41 08          	movups %xmm0,0x8(%rcx)
   298e1:	c6 01 ff             	movb   $0xff,(%rcx)
   298e4:	48 8b b4 24 f0 00 00 	mov    0xf0(%rsp),%rsi
   298eb:	00 
   298ec:	48 85 f6             	test   %rsi,%rsi
   298ef:	74 17                	je     29908 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1f38>
   298f1:	48 c1 e6 03          	shl    $0x3,%rsi
   298f5:	ba 08 00 00 00       	mov    $0x8,%edx
   298fa:	48 8b bc 24 60 01 00 	mov    0x160(%rsp),%rdi
   29901:	00 
   29902:	ff 15 38 9e 04 00    	call   *0x49e38(%rip)        # 73740 <_DYNAMIC+0x250>
   29908:	48 8b 74 24 10       	mov    0x10(%rsp),%rsi
   2990d:	48 85 f6             	test   %rsi,%rsi
   29910:	74 14                	je     29926 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1f56>
   29912:	48 8b 7c 24 18       	mov    0x18(%rsp),%rdi
   29917:	48 c1 e6 03          	shl    $0x3,%rsi
   2991b:	ba 08 00 00 00       	mov    $0x8,%edx
   29920:	ff 15 1a 9e 04 00    	call   *0x49e1a(%rip)        # 73740 <_DYNAMIC+0x250>
   29926:	be 00 8c 0a 00       	mov    $0xa8c00,%esi
   2992b:	ba 08 00 00 00       	mov    $0x8,%edx
   29930:	48 8b 7c 24 78       	mov    0x78(%rsp),%rdi
   29935:	ff 15 05 9e 04 00    	call   *0x49e05(%rip)        # 73740 <_DYNAMIC+0x250>
   2993b:	48 8b bc 24 20 01 00 	mov    0x120(%rsp),%rdi
   29942:	00 
   29943:	e9 ec e1 ff ff       	jmp    27b34 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x164>
   29948:	48 89 c7             	mov    %rax,%rdi
   2994b:	48 8d 15 7e 78 04 00 	lea    0x4787e(%rip),%rdx        # 711d0 <anon.22e4ac0f336c447f4ab0ae585f38cc41.18.llvm.10198508844383412456+0x210>
   29952:	e9 0a ee ff ff       	jmp    28761 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0xd91>
   29957:	48 89 c7             	mov    %rax,%rdi
   2995a:	48 8d 15 87 78 04 00 	lea    0x47887(%rip),%rdx        # 711e8 <anon.22e4ac0f336c447f4ab0ae585f38cc41.18.llvm.10198508844383412456+0x228>
   29961:	48 8b 9c 24 90 00 00 	mov    0x90(%rsp),%rbx
   29968:	00 
   29969:	e9 f3 ed ff ff       	jmp    28761 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0xd91>
   2996e:	48 8d 15 43 78 04 00 	lea    0x47843(%rip),%rdx        # 711b8 <anon.22e4ac0f336c447f4ab0ae585f38cc41.18.llvm.10198508844383412456+0x1f8>
   29975:	e9 e7 ed ff ff       	jmp    28761 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0xd91>
   2997a:	bf 08 00 00 00       	mov    $0x8,%edi
   2997f:	be 50 00 00 00       	mov    $0x50,%esi
   29984:	ff 15 e6 9d 04 00    	call   *0x49de6(%rip)        # 73770 <_DYNAMIC+0x280>
   2998a:	eb 2e                	jmp    299ba <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1fea>
   2998c:	4c 89 ef             	mov    %r13,%rdi
   2998f:	4c 89 fe             	mov    %r15,%rsi
   29992:	ff 15 d8 9d 04 00    	call   *0x49dd8(%rip)        # 73770 <_DYNAMIC+0x280>
   29998:	eb 20                	jmp    299ba <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1fea>
   2999a:	bf 08 00 00 00       	mov    $0x8,%edi
   2999f:	be 00 8c 0a 00       	mov    $0xa8c00,%esi
   299a4:	ff 15 c6 9d 04 00    	call   *0x49dc6(%rip)        # 73770 <_DYNAMIC+0x280>
   299aa:	bf 08 00 00 00       	mov    $0x8,%edi
   299af:	be 00 2a 03 00       	mov    $0x32a00,%esi
   299b4:	ff 15 b6 9d 04 00    	call   *0x49db6(%rip)        # 73770 <_DYNAMIC+0x280>
   299ba:	0f 0b                	ud2
   299bc:	eb 2b                	jmp    299e9 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x2019>
   299be:	49 89 c4             	mov    %rax,%r12
   299c1:	e9 d8 00 00 00       	jmp    29a9e <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x20ce>
   299c6:	e9 b2 00 00 00       	jmp    29a7d <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x20ad>
   299cb:	49 89 c4             	mov    %rax,%r12
   299ce:	e9 82 00 00 00       	jmp    29a55 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x2085>
   299d3:	e9 a5 00 00 00       	jmp    29a7d <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x20ad>
   299d8:	49 89 c4             	mov    %rax,%r12
   299db:	4c 8b 74 24 08       	mov    0x8(%rsp),%r14
   299e0:	eb 3d                	jmp    29a1f <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x204f>
   299e2:	eb 5b                	jmp    29a3f <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x206f>
   299e4:	49 89 c4             	mov    %rax,%r12
   299e7:	eb 36                	jmp    29a1f <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x204f>
   299e9:	49 89 c4             	mov    %rax,%r12
   299ec:	48 8b 74 24 28       	mov    0x28(%rsp),%rsi
   299f1:	48 85 f6             	test   %rsi,%rsi
   299f4:	74 5f                	je     29a55 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x2085>
   299f6:	48 8b 7c 24 30       	mov    0x30(%rsp),%rdi
   299fb:	ba 01 00 00 00       	mov    $0x1,%edx
   29a00:	eb 4d                	jmp    29a4f <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x207f>
   29a02:	49 89 c4             	mov    %rax,%r12
   29a05:	4d 85 ff             	test   %r15,%r15
   29a08:	74 15                	je     29a1f <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x204f>
   29a0a:	49 c1 e7 03          	shl    $0x3,%r15
   29a0e:	ba 08 00 00 00       	mov    $0x8,%edx
   29a13:	4c 89 ef             	mov    %r13,%rdi
   29a16:	4c 89 fe             	mov    %r15,%rsi
   29a19:	ff 15 21 9d 04 00    	call   *0x49d21(%rip)        # 73740 <_DYNAMIC+0x250>
   29a1f:	48 8b 74 24 28       	mov    0x28(%rsp),%rsi
   29a24:	48 85 f6             	test   %rsi,%rsi
   29a27:	74 19                	je     29a42 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x2072>
   29a29:	48 8b 7c 24 30       	mov    0x30(%rsp),%rdi
   29a2e:	48 c1 e6 03          	shl    $0x3,%rsi
   29a32:	ba 08 00 00 00       	mov    $0x8,%edx
   29a37:	ff 15 03 9d 04 00    	call   *0x49d03(%rip)        # 73740 <_DYNAMIC+0x250>
   29a3d:	eb 03                	jmp    29a42 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x2072>
   29a3f:	49 89 c4             	mov    %rax,%r12
   29a42:	be 20 03 00 00       	mov    $0x320,%esi
   29a47:	ba 08 00 00 00       	mov    $0x8,%edx
   29a4c:	4c 89 f7             	mov    %r14,%rdi
   29a4f:	ff 15 eb 9c 04 00    	call   *0x49ceb(%rip)        # 73740 <_DYNAMIC+0x250>
   29a55:	48 8b b4 24 f0 00 00 	mov    0xf0(%rsp),%rsi
   29a5c:	00 
   29a5d:	48 85 f6             	test   %rsi,%rsi
   29a60:	74 1e                	je     29a80 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x20b0>
   29a62:	48 8b bc 24 f8 00 00 	mov    0xf8(%rsp),%rdi
   29a69:	00 
   29a6a:	48 c1 e6 03          	shl    $0x3,%rsi
   29a6e:	ba 08 00 00 00       	mov    $0x8,%edx
   29a73:	ff 15 c7 9c 04 00    	call   *0x49cc7(%rip)        # 73740 <_DYNAMIC+0x250>
   29a79:	eb 05                	jmp    29a80 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x20b0>
   29a7b:	eb 00                	jmp    29a7d <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x20ad>
   29a7d:	49 89 c4             	mov    %rax,%r12
   29a80:	48 8b 74 24 10       	mov    0x10(%rsp),%rsi
   29a85:	48 85 f6             	test   %rsi,%rsi
   29a88:	74 14                	je     29a9e <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x20ce>
   29a8a:	48 8b 7c 24 18       	mov    0x18(%rsp),%rdi
   29a8f:	48 c1 e6 03          	shl    $0x3,%rsi
   29a93:	ba 08 00 00 00       	mov    $0x8,%edx
   29a98:	ff 15 a2 9c 04 00    	call   *0x49ca2(%rip)        # 73740 <_DYNAMIC+0x250>
   29a9e:	be 00 8c 0a 00       	mov    $0xa8c00,%esi
   29aa3:	ba 08 00 00 00       	mov    $0x8,%edx
   29aa8:	48 8b 7c 24 78       	mov    0x78(%rsp),%rdi
   29aad:	ff 15 8d 9c 04 00    	call   *0x49c8d(%rip)        # 73740 <_DYNAMIC+0x250>
   29ab3:	4c 89 e7             	mov    %r12,%rdi
   29ab6:	e8 e5 5c 04 00       	call   6f7a0 <_Unwind_Resume@plt>
