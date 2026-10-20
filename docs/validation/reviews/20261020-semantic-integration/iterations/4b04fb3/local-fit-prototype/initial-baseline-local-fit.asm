
/tmp/xdremux-style-fit-probe-target/release/xdremux-style-fit-probe:     file format elf64-x86-64


Disassembly of section .text:

0000000000024ce0 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data>:
   24ce0:	55                   	push   %rbp
   24ce1:	41 57                	push   %r15
   24ce3:	41 56                	push   %r14
   24ce5:	41 55                	push   %r13
   24ce7:	41 54                	push   %r12
   24ce9:	53                   	push   %rbx
   24cea:	48 81 ec 68 02 00 00 	sub    $0x268,%rsp
   24cf1:	4c 8b 76 20          	mov    0x20(%rsi),%r14
   24cf5:	48 8b 4e 28          	mov    0x28(%rsi),%rcx
   24cf9:	4c 89 f0             	mov    %r14,%rax
   24cfc:	48 f7 e1             	mul    %rcx
   24cff:	0f 80 3c 01 00 00    	jo     24e41 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x161>
   24d05:	48 89 8c 24 40 01 00 	mov    %rcx,0x140(%rsp)
   24d0c:	00 
   24d0d:	b9 03 00 00 00       	mov    $0x3,%ecx
   24d12:	48 f7 e1             	mul    %rcx
   24d15:	0f 80 26 01 00 00    	jo     24e41 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x161>
   24d1b:	4d 85 f6             	test   %r14,%r14
   24d1e:	0f 84 1d 01 00 00    	je     24e41 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x161>
   24d24:	48 83 bc 24 40 01 00 	cmpq   $0x0,0x140(%rsp)
   24d2b:	00 00 
   24d2d:	0f 84 0e 01 00 00    	je     24e41 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x161>
   24d33:	48 89 c3             	mov    %rax,%rbx
   24d36:	48 39 46 08          	cmp    %rax,0x8(%rsi)
   24d3a:	0f 85 01 01 00 00    	jne    24e41 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x161>
   24d40:	48 39 5e 18          	cmp    %rbx,0x18(%rsi)
   24d44:	0f 85 f7 00 00 00    	jne    24e41 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x161>
   24d4a:	4c 8b 3e             	mov    (%rsi),%r15
   24d4d:	48 8d 04 9d 00 00 00 	lea    0x0(,%rbx,4),%rax
   24d54:	00 
   24d55:	48 85 db             	test   %rbx,%rbx
   24d58:	74 21                	je     24d7b <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x9b>
   24d5a:	31 c9                	xor    %ecx,%ecx
   24d5c:	0f 1f 40 00          	nopl   0x0(%rax)
   24d60:	41 8b 14 0f          	mov    (%r15,%rcx,1),%edx
   24d64:	01 d2                	add    %edx,%edx
   24d66:	81 fa ff ff ff fe    	cmp    $0xfeffffff,%edx
   24d6c:	0f 87 cf 00 00 00    	ja     24e41 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x161>
   24d72:	48 83 c1 04          	add    $0x4,%rcx
   24d76:	48 39 c8             	cmp    %rcx,%rax
   24d79:	75 e5                	jne    24d60 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x80>
   24d7b:	4c 8b 66 10          	mov    0x10(%rsi),%r12
   24d7f:	48 85 db             	test   %rbx,%rbx
   24d82:	74 27                	je     24dab <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0xcb>
   24d84:	31 c9                	xor    %ecx,%ecx
   24d86:	66 2e 0f 1f 84 00 00 	cs nopw 0x0(%rax,%rax,1)
   24d8d:	00 00 00 
   24d90:	41 8b 14 0c          	mov    (%r12,%rcx,1),%edx
   24d94:	01 d2                	add    %edx,%edx
   24d96:	81 fa ff ff ff fe    	cmp    $0xfeffffff,%edx
   24d9c:	0f 87 9f 00 00 00    	ja     24e41 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x161>
   24da2:	48 83 c1 04          	add    $0x4,%rcx
   24da6:	48 39 c8             	cmp    %rcx,%rax
   24da9:	75 e5                	jne    24d90 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0xb0>
   24dab:	48 89 bc 24 20 01 00 	mov    %rdi,0x120(%rsp)
   24db2:	00 
   24db3:	ff 15 6f ab 04 00    	call   *0x4ab6f(%rip)        # 6f928 <_DYNAMIC+0x2b8>
   24db9:	bf 00 8c 0a 00       	mov    $0xa8c00,%edi
   24dbe:	be 08 00 00 00       	mov    $0x8,%esi
   24dc3:	ff 15 7f ac 04 00    	call   *0x4ac7f(%rip)        # 6fa48 <_DYNAMIC+0x3d8>
   24dc9:	48 89 44 24 78       	mov    %rax,0x78(%rsp)
   24dce:	48 85 c0             	test   %rax,%rax
   24dd1:	0f 84 d3 1e 00 00    	je     26caa <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1fca>
   24dd7:	ff 15 4b ab 04 00    	call   *0x4ab4b(%rip)        # 6f928 <_DYNAMIC+0x2b8>
   24ddd:	bf 00 2a 03 00       	mov    $0x32a00,%edi
   24de2:	be 08 00 00 00       	mov    $0x8,%esi
   24de7:	ff 15 5b ac 04 00    	call   *0x4ac5b(%rip)        # 6fa48 <_DYNAMIC+0x3d8>
   24ded:	48 85 c0             	test   %rax,%rax
   24df0:	0f 84 c4 1e 00 00    	je     26cba <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1fda>
   24df6:	48 c7 44 24 10 40 65 	movq   $0x6540,0x10(%rsp)
   24dfd:	00 00 
   24dff:	48 89 44 24 18       	mov    %rax,0x18(%rsp)
   24e04:	48 c7 44 24 20 40 65 	movq   $0x6540,0x20(%rsp)
   24e0b:	00 00 
   24e0d:	31 ed                	xor    %ebp,%ebp
   24e0f:	4c 8d ac 24 f0 00 00 	lea    0xf0(%rsp),%r13
   24e16:	00 
   24e17:	48 8b 94 24 40 01 00 	mov    0x140(%rsp),%rdx
   24e1e:	00 
   24e1f:	48 89 9c 24 90 00 00 	mov    %rbx,0x90(%rsp)
   24e26:	00 
   24e27:	4c 89 b4 24 18 02 00 	mov    %r14,0x218(%rsp)
   24e2e:	00 
   24e2f:	4c 89 bc 24 10 02 00 	mov    %r15,0x210(%rsp)
   24e36:	00 
   24e37:	4c 89 a4 24 08 02 00 	mov    %r12,0x208(%rsp)
   24e3e:	00 
   24e3f:	eb 3e                	jmp    24e7f <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x19f>
   24e41:	c6 07 02             	movb   $0x2,(%rdi)
   24e44:	48 89 f8             	mov    %rdi,%rax
   24e47:	48 81 c4 68 02 00 00 	add    $0x268,%rsp
   24e4e:	5b                   	pop    %rbx
   24e4f:	41 5c                	pop    %r12
   24e51:	41 5d                	pop    %r13
   24e53:	41 5e                	pop    %r14
   24e55:	41 5f                	pop    %r15
   24e57:	5d                   	pop    %rbp
   24e58:	c3                   	ret
   24e59:	0f 1f 80 00 00 00 00 	nopl   0x0(%rax)
   24e60:	49 89 d7             	mov    %rdx,%r15
   24e63:	48 8b 84 24 00 02 00 	mov    0x200(%rsp),%rax
   24e6a:	00 
   24e6b:	48 89 c5             	mov    %rax,%rbp
   24e6e:	48 8b 94 24 40 01 00 	mov    0x140(%rsp),%rdx
   24e75:	00 
   24e76:	48 39 d0             	cmp    %rdx,%rax
   24e79:	0f 84 00 0c 00 00    	je     25a7f <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0xd9f>
   24e7f:	b9 09 00 00 00       	mov    $0x9,%ecx
   24e84:	4c 89 ef             	mov    %r13,%rdi
   24e87:	48 89 ee             	mov    %rbp,%rsi
   24e8a:	e8 01 ec ff ff       	call   23a90 <xdremux_engine::apple_photographic_styles::spotlight_weights>
   24e8f:	48 8d 45 01          	lea    0x1(%rbp),%rax
   24e93:	48 89 84 24 00 02 00 	mov    %rax,0x200(%rsp)
   24e9a:	00 
   24e9b:	49 0f af ee          	imul   %r14,%rbp
   24e9f:	48 8b 84 24 f0 00 00 	mov    0xf0(%rsp),%rax
   24ea6:	00 
   24ea7:	f2 0f 10 84 24 f8 00 	movsd  0xf8(%rsp),%xmm0
   24eae:	00 00 
   24eb0:	f2 0f 11 84 24 28 01 	movsd  %xmm0,0x128(%rsp)
   24eb7:	00 00 
   24eb9:	48 8b 8c 24 00 01 00 	mov    0x100(%rsp),%rcx
   24ec0:	00 
   24ec1:	f2 0f 10 84 24 08 01 	movsd  0x108(%rsp),%xmm0
   24ec8:	00 00 
   24eca:	f2 0f 11 84 24 c8 01 	movsd  %xmm0,0x1c8(%rsp)
   24ed1:	00 00 
   24ed3:	48 69 d0 20 03 00 00 	imul   $0x320,%rax,%rdx
   24eda:	48 89 94 24 b8 01 00 	mov    %rdx,0x1b8(%rsp)
   24ee1:	00 
   24ee2:	48 89 84 24 c0 01 00 	mov    %rax,0x1c0(%rsp)
   24ee9:	00 
   24eea:	48 69 d0 00 19 00 00 	imul   $0x1900,%rax,%rdx
   24ef1:	48 8b 44 24 78       	mov    0x78(%rsp),%rax
   24ef6:	48 01 c2             	add    %rax,%rdx
   24ef9:	48 89 94 24 b0 01 00 	mov    %rdx,0x1b0(%rsp)
   24f00:	00 
   24f01:	48 69 d1 20 03 00 00 	imul   $0x320,%rcx,%rdx
   24f08:	48 89 94 24 a0 01 00 	mov    %rdx,0x1a0(%rsp)
   24f0f:	00 
   24f10:	48 89 8c 24 a8 01 00 	mov    %rcx,0x1a8(%rsp)
   24f17:	00 
   24f18:	48 69 c9 00 19 00 00 	imul   $0x1900,%rcx,%rcx
   24f1f:	48 01 c1             	add    %rax,%rcx
   24f22:	48 89 8c 24 98 01 00 	mov    %rcx,0x198(%rsp)
   24f29:	00 
   24f2a:	31 c9                	xor    %ecx,%ecx
   24f2c:	4c 89 fa             	mov    %r15,%rdx
   24f2f:	48 89 6c 24 08       	mov    %rbp,0x8(%rsp)
   24f34:	eb 3b                	jmp    24f71 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x291>
   24f36:	66 2e 0f 1f 84 00 00 	cs nopw 0x0(%rax,%rax,1)
   24f3d:	00 00 00 
   24f40:	48 8b 9c 24 90 00 00 	mov    0x90(%rsp),%rbx
   24f47:	00 
   24f48:	4c 8b b4 24 18 02 00 	mov    0x218(%rsp),%r14
   24f4f:	00 
   24f50:	48 8b 94 24 10 02 00 	mov    0x210(%rsp),%rdx
   24f57:	00 
   24f58:	4c 8b a4 24 08 02 00 	mov    0x208(%rsp),%r12
   24f5f:	00 
   24f60:	48 8b 4c 24 70       	mov    0x70(%rsp),%rcx
   24f65:	48 ff c1             	inc    %rcx
   24f68:	4c 39 f1             	cmp    %r14,%rcx
   24f6b:	0f 84 ef fe ff ff    	je     24e60 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x180>
   24f71:	48 8d 04 29          	lea    (%rcx,%rbp,1),%rax
   24f75:	48 8d 3c 40          	lea    (%rax,%rax,2),%rdi
   24f79:	48 39 df             	cmp    %rbx,%rdi
   24f7c:	0f 83 fc 1c 00 00    	jae    26c7e <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1f9e>
   24f82:	48 8d 47 01          	lea    0x1(%rdi),%rax
   24f86:	48 39 d8             	cmp    %rbx,%rax
   24f89:	0f 83 c9 1c 00 00    	jae    26c58 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1f78>
   24f8f:	48 89 4c 24 70       	mov    %rcx,0x70(%rsp)
   24f94:	48 8d 47 02          	lea    0x2(%rdi),%rax
   24f98:	48 39 d8             	cmp    %rbx,%rax
   24f9b:	0f 83 c6 1c 00 00    	jae    26c67 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1f87>
   24fa1:	4d 89 f7             	mov    %r14,%r15
   24fa4:	f3 0f 10 04 ba       	movss  (%rdx,%rdi,4),%xmm0
   24fa9:	0f 57 e4             	xorps  %xmm4,%xmm4
   24fac:	f3 0f 5a e0          	cvtss2sd %xmm0,%xmm4
   24fb0:	f3 41 0f 10 04 bc    	movss  (%r12,%rdi,4),%xmm0
   24fb6:	f3 0f 11 84 24 58 01 	movss  %xmm0,0x158(%rsp)
   24fbd:	00 00 
   24fbf:	f3 41 0f 10 44 bc 04 	movss  0x4(%r12,%rdi,4),%xmm0
   24fc6:	f3 0f 11 84 24 50 01 	movss  %xmm0,0x150(%rsp)
   24fcd:	00 00 
   24fcf:	f3 41 0f 10 44 bc 08 	movss  0x8(%r12,%rdi,4),%xmm0
   24fd6:	f3 0f 11 84 24 48 01 	movss  %xmm0,0x148(%rsp)
   24fdd:	00 00 
   24fdf:	0f 28 c4             	movaps %xmm4,%xmm0
   24fe2:	48 b8 00 00 00 00 00 	movabs $0x3ff0000000000000,%rax
   24fe9:	00 f0 3f 
   24fec:	48 89 84 24 a0 00 00 	mov    %rax,0xa0(%rsp)
   24ff3:	00 
   24ff4:	f2 0f 11 a4 24 a8 00 	movsd  %xmm4,0xa8(%rsp)
   24ffb:	00 00 
   24ffd:	0f 5a 54 ba 04       	cvtps2pd 0x4(%rdx,%rdi,4),%xmm2
   25002:	f2 0f 59 c4          	mulsd  %xmm4,%xmm0
   25006:	66 0f 28 ea          	movapd %xmm2,%xmm5
   2500a:	66 0f 15 ea          	unpckhpd %xmm2,%xmm5
   2500e:	66 0f 28 cd          	movapd %xmm5,%xmm1
   25012:	f2 0f 59 cd          	mulsd  %xmm5,%xmm1
   25016:	66 0f 11 94 24 b0 00 	movupd %xmm2,0xb0(%rsp)
   2501d:	00 00 
   2501f:	f2 0f 11 84 24 c0 00 	movsd  %xmm0,0xc0(%rsp)
   25026:	00 00 
   25028:	66 0f 28 c4          	movapd %xmm4,%xmm0
   2502c:	66 0f 14 c4          	unpcklpd %xmm4,%xmm0
   25030:	66 0f 59 c2          	mulpd  %xmm2,%xmm0
   25034:	66 0f 11 84 24 c8 00 	movupd %xmm0,0xc8(%rsp)
   2503b:	00 00 
   2503d:	66 0f 28 c2          	movapd %xmm2,%xmm0
   25041:	66 0f 14 c2          	unpcklpd %xmm2,%xmm0
   25045:	66 0f 59 c2          	mulpd  %xmm2,%xmm0
   25049:	66 0f 11 84 24 d8 00 	movupd %xmm0,0xd8(%rsp)
   25050:	00 00 
   25052:	f2 0f 11 8c 24 e8 00 	movsd  %xmm1,0xe8(%rsp)
   25059:	00 00 
   2505b:	66 0f 28 c2          	movapd %xmm2,%xmm0
   2505f:	f2 0f 58 c4          	addsd  %xmm4,%xmm0
   25063:	f2 0f 58 c5          	addsd  %xmm5,%xmm0
   25067:	f2 0f 5e 05 99 7d fe 	divsd  -0x18267(%rip),%xmm0        # ce08 <GCC_except_table0+0x720>
   2506e:	ff 
   2506f:	66 0f 28 cc          	movapd %xmm4,%xmm1
   25073:	f2 0f 5f ca          	maxsd  %xmm2,%xmm1
   25077:	66 0f 29 94 24 d0 01 	movapd %xmm2,0x1d0(%rsp)
   2507e:	00 00 
   25080:	f2 0f c2 d2 03       	cmpunordsd %xmm2,%xmm2
   25085:	66 0f 28 da          	movapd %xmm2,%xmm3
   25089:	66 0f 55 d9          	andnpd %xmm1,%xmm3
   2508d:	0f 29 a4 24 f0 01 00 	movaps %xmm4,0x1f0(%rsp)
   25094:	00 
   25095:	66 0f 54 d4          	andpd  %xmm4,%xmm2
   25099:	66 0f 56 d3          	orpd   %xmm3,%xmm2
   2509d:	66 0f 28 dd          	movapd %xmm5,%xmm3
   250a1:	f2 0f c2 dd 03       	cmpunordsd %xmm5,%xmm3
   250a6:	66 0f 28 cb          	movapd %xmm3,%xmm1
   250aa:	66 0f 54 ca          	andpd  %xmm2,%xmm1
   250ae:	66 0f 29 ac 24 e0 01 	movapd %xmm5,0x1e0(%rsp)
   250b5:	00 00 
   250b7:	f2 0f 5f d5          	maxsd  %xmm5,%xmm2
   250bb:	66 0f 55 da          	andnpd %xmm2,%xmm3
   250bf:	66 0f 56 d9          	orpd   %xmm1,%xmm3
   250c3:	f2 0f 58 d8          	addsd  %xmm0,%xmm3
   250c7:	f2 0f 59 1d 21 7d fe 	mulsd  -0x182df(%rip),%xmm3        # cdf0 <GCC_except_table0+0x708>
   250ce:	ff 
   250cf:	66 0f 14 db          	unpcklpd %xmm3,%xmm3
   250d3:	66 0f 29 5c 24 60    	movapd %xmm3,0x60(%rsp)
   250d9:	66 0f 58 1d 8f 78 fe 	addpd  -0x18771(%rip),%xmm3        # c970 <GCC_except_table0+0x288>
   250e0:	ff 
   250e1:	66 0f 28 05 f7 77 fe 	movapd -0x18809(%rip),%xmm0        # c8e0 <GCC_except_table0+0x1f8>
   250e8:	ff 
   250e9:	66 0f 59 d8          	mulpd  %xmm0,%xmm3
   250ed:	66 0f 28 05 db 7a fe 	movapd -0x18525(%rip),%xmm0        # cbd0 <GCC_except_table0+0x4e8>
   250f4:	ff 
   250f5:	66 0f 5e d8          	divpd  %xmm0,%xmm3
   250f9:	66 0f 59 db          	mulpd  %xmm3,%xmm3
   250fd:	66 0f 28 05 2b 7c fe 	movapd -0x183d5(%rip),%xmm0        # cd30 <GCC_except_table0+0x648>
   25104:	ff 
   25105:	66 0f 59 d8          	mulpd  %xmm0,%xmm3
   25109:	66 0f 29 9c 24 80 00 	movapd %xmm3,0x80(%rsp)
   25110:	00 00 
   25112:	66 0f 28 c3          	movapd %xmm3,%xmm0
   25116:	4c 8b 35 43 ab 04 00 	mov    0x4ab43(%rip),%r14        # 6fc60 <exp@GLIBC_2.29>
   2511d:	48 89 d3             	mov    %rdx,%rbx
   25120:	41 ff d6             	call   *%r14
   25123:	66 0f 29 44 24 50    	movapd %xmm0,0x50(%rsp)
   25129:	0f 28 84 24 80 00 00 	movaps 0x80(%rsp),%xmm0
   25130:	00 
   25131:	0f 12 c0             	movhlps %xmm0,%xmm0
   25134:	41 ff d6             	call   *%r14
   25137:	0f 29 84 24 80 01 00 	movaps %xmm0,0x180(%rsp)
   2513e:	00 
   2513f:	0f 28 4c 24 50       	movaps 0x50(%rsp),%xmm1
   25144:	0f 16 c8             	movlhps %xmm0,%xmm1
   25147:	0f 29 4c 24 50       	movaps %xmm1,0x50(%rsp)
   2514c:	0f 29 8c 24 20 02 00 	movaps %xmm1,0x220(%rsp)
   25153:	00 
   25154:	66 0f 28 44 24 60    	movapd 0x60(%rsp),%xmm0
   2515a:	66 0f 58 05 de 78 fe 	addpd  -0x18722(%rip),%xmm0        # ca40 <GCC_except_table0+0x358>
   25161:	ff 
   25162:	66 0f 59 05 76 77 fe 	mulpd  -0x1888a(%rip),%xmm0        # c8e0 <GCC_except_table0+0x1f8>
   25169:	ff 
   2516a:	66 0f 5e 05 5e 7a fe 	divpd  -0x185a2(%rip),%xmm0        # cbd0 <GCC_except_table0+0x4e8>
   25171:	ff 
   25172:	66 0f 59 c0          	mulpd  %xmm0,%xmm0
   25176:	66 0f 59 05 b2 7b fe 	mulpd  -0x1844e(%rip),%xmm0        # cd30 <GCC_except_table0+0x648>
   2517d:	ff 
   2517e:	66 0f 29 84 24 10 01 	movapd %xmm0,0x110(%rsp)
   25185:	00 00 
   25187:	41 ff d6             	call   *%r14
   2518a:	66 0f 29 84 24 80 00 	movapd %xmm0,0x80(%rsp)
   25191:	00 00 
   25193:	0f 28 84 24 10 01 00 	movaps 0x110(%rsp),%xmm0
   2519a:	00 
   2519b:	0f 12 c0             	movhlps %xmm0,%xmm0
   2519e:	41 ff d6             	call   *%r14
   251a1:	0f 29 84 24 70 01 00 	movaps %xmm0,0x170(%rsp)
   251a8:	00 
   251a9:	0f 28 8c 24 80 00 00 	movaps 0x80(%rsp),%xmm1
   251b0:	00 
   251b1:	0f 16 c8             	movlhps %xmm0,%xmm1
   251b4:	0f 29 8c 24 80 00 00 	movaps %xmm1,0x80(%rsp)
   251bb:	00 
   251bc:	0f 29 8c 24 30 02 00 	movaps %xmm1,0x230(%rsp)
   251c3:	00 
   251c4:	66 0f 28 44 24 60    	movapd 0x60(%rsp),%xmm0
   251ca:	66 0f 58 05 4e 76 fe 	addpd  -0x189b2(%rip),%xmm0        # c820 <GCC_except_table0+0x138>
   251d1:	ff 
   251d2:	66 0f 59 05 06 77 fe 	mulpd  -0x188fa(%rip),%xmm0        # c8e0 <GCC_except_table0+0x1f8>
   251d9:	ff 
   251da:	66 0f 5e 05 ee 79 fe 	divpd  -0x18612(%rip),%xmm0        # cbd0 <GCC_except_table0+0x4e8>
   251e1:	ff 
   251e2:	66 0f 59 c0          	mulpd  %xmm0,%xmm0
   251e6:	66 0f 59 05 42 7b fe 	mulpd  -0x184be(%rip),%xmm0        # cd30 <GCC_except_table0+0x648>
   251ed:	ff 
   251ee:	66 0f 29 84 24 30 01 	movapd %xmm0,0x130(%rsp)
   251f5:	00 00 
   251f7:	41 ff d6             	call   *%r14
   251fa:	66 0f 29 84 24 10 01 	movapd %xmm0,0x110(%rsp)
   25201:	00 00 
   25203:	0f 28 84 24 30 01 00 	movaps 0x130(%rsp),%xmm0
   2520a:	00 
   2520b:	0f 12 c0             	movhlps %xmm0,%xmm0
   2520e:	41 ff d6             	call   *%r14
   25211:	0f 29 84 24 30 01 00 	movaps %xmm0,0x130(%rsp)
   25218:	00 
   25219:	0f 28 8c 24 10 01 00 	movaps 0x110(%rsp),%xmm1
   25220:	00 
   25221:	0f 16 c8             	movlhps %xmm0,%xmm1
   25224:	0f 29 8c 24 10 01 00 	movaps %xmm1,0x110(%rsp)
   2522b:	00 
   2522c:	0f 29 8c 24 40 02 00 	movaps %xmm1,0x240(%rsp)
   25233:	00 
   25234:	66 0f 28 44 24 60    	movapd 0x60(%rsp),%xmm0
   2523a:	66 0f 58 05 ee 78 fe 	addpd  -0x18712(%rip),%xmm0        # cb30 <GCC_except_table0+0x448>
   25241:	ff 
   25242:	66 0f 59 05 96 76 fe 	mulpd  -0x1896a(%rip),%xmm0        # c8e0 <GCC_except_table0+0x1f8>
   25249:	ff 
   2524a:	66 0f 5e 05 7e 79 fe 	divpd  -0x18682(%rip),%xmm0        # cbd0 <GCC_except_table0+0x4e8>
   25251:	ff 
   25252:	66 0f 59 c0          	mulpd  %xmm0,%xmm0
   25256:	66 0f 59 05 d2 7a fe 	mulpd  -0x1852e(%rip),%xmm0        # cd30 <GCC_except_table0+0x648>
   2525d:	ff 
   2525e:	66 0f 29 44 24 60    	movapd %xmm0,0x60(%rsp)
   25264:	41 ff d6             	call   *%r14
   25267:	66 0f 29 84 24 60 01 	movapd %xmm0,0x160(%rsp)
   2526e:	00 00 
   25270:	66 0f 28 44 24 60    	movapd 0x60(%rsp),%xmm0
   25276:	66 0f 15 c0          	unpckhpd %xmm0,%xmm0
   2527a:	41 ff d6             	call   *%r14
   2527d:	48 89 da             	mov    %rbx,%rdx
   25280:	66 0f 28 8c 24 60 01 	movapd 0x160(%rsp),%xmm1
   25287:	00 00 
   25289:	66 0f 14 c8          	unpcklpd %xmm0,%xmm1
   2528d:	66 0f 29 8c 24 50 02 	movapd %xmm1,0x250(%rsp)
   25294:	00 00 
   25296:	66 0f 28 d1          	movapd %xmm1,%xmm2
   2529a:	66 0f 28 4c 24 50    	movapd 0x50(%rsp),%xmm1
   252a0:	f2 0f 58 8c 24 80 01 	addsd  0x180(%rsp),%xmm1
   252a7:	00 00 
   252a9:	f2 0f 58 8c 24 80 00 	addsd  0x80(%rsp),%xmm1
   252b0:	00 00 
   252b2:	f2 0f 58 8c 24 70 01 	addsd  0x170(%rsp),%xmm1
   252b9:	00 00 
   252bb:	f2 0f 58 8c 24 10 01 	addsd  0x110(%rsp),%xmm1
   252c2:	00 00 
   252c4:	f2 0f 58 8c 24 30 01 	addsd  0x130(%rsp),%xmm1
   252cb:	00 00 
   252cd:	f2 0f 58 ca          	addsd  %xmm2,%xmm1
   252d1:	f2 0f 58 c8          	addsd  %xmm0,%xmm1
   252d5:	66 0f 57 c0          	xorpd  %xmm0,%xmm0
   252d9:	66 0f 29 4c 24 50    	movapd %xmm1,0x50(%rsp)
   252df:	66 0f 2e c1          	ucomisd %xmm1,%xmm0
   252e3:	4d 89 fe             	mov    %r15,%r14
   252e6:	48 8b 9c 24 90 00 00 	mov    0x90(%rsp),%rbx
   252ed:	00 
   252ee:	0f 83 6c fc ff ff    	jae    24f60 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x280>
   252f4:	b9 0c 00 00 00       	mov    $0xc,%ecx
   252f9:	48 8d 7c 24 28       	lea    0x28(%rsp),%rdi
   252fe:	48 8b 74 24 70       	mov    0x70(%rsp),%rsi
   25303:	4c 89 f2             	mov    %r14,%rdx
   25306:	e8 85 e7 ff ff       	call   23a90 <xdremux_engine::apple_photographic_styles::spotlight_weights>
   2530b:	f3 0f 10 84 24 58 01 	movss  0x158(%rsp),%xmm0
   25312:	00 00 
   25314:	f3 0f 5a c0          	cvtss2sd %xmm0,%xmm0
   25318:	f3 0f 10 8c 24 50 01 	movss  0x150(%rsp),%xmm1
   2531f:	00 00 
   25321:	f3 0f 5a c9          	cvtss2sd %xmm1,%xmm1
   25325:	f3 0f 10 94 24 48 01 	movss  0x148(%rsp),%xmm2
   2532c:	00 00 
   2532e:	f3 0f 5a d2          	cvtss2sd %xmm2,%xmm2
   25332:	f2 0f 10 64 24 30    	movsd  0x30(%rsp),%xmm4
   25338:	48 8b 44 24 28       	mov    0x28(%rsp),%rax
   2533d:	48 89 84 24 80 00 00 	mov    %rax,0x80(%rsp)
   25344:	00 
   25345:	48 8d 0c c0          	lea    (%rax,%rax,8),%rcx
   25349:	f2 0f 5c 84 24 f0 01 	subsd  0x1f0(%rsp),%xmm0
   25350:	00 00 
   25352:	f2 0f 5c 8c 24 d0 01 	subsd  0x1d0(%rsp),%xmm1
   25359:	00 00 
   2535b:	f2 0f 5c 94 24 e0 01 	subsd  0x1e0(%rsp),%xmm2
   25362:	00 00 
   25364:	f2 0f 10 5c 24 40    	movsd  0x40(%rsp),%xmm3
   2536a:	48 8b 44 24 38       	mov    0x38(%rsp),%rax
   2536f:	48 89 44 24 60       	mov    %rax,0x60(%rsp)
   25374:	66 0f 28 ec          	movapd %xmm4,%xmm5
   25378:	f2 0f 59 ac 24 28 01 	mulsd  0x128(%rsp),%xmm5
   2537f:	00 00 
   25381:	66 0f 57 f6          	xorpd  %xmm6,%xmm6
   25385:	66 0f 2e f5          	ucomisd %xmm5,%xmm6
   25389:	0f 83 a1 01 00 00    	jae    25530 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x850>
   2538f:	48 8b 84 24 c0 01 00 	mov    0x1c0(%rsp),%rax
   25396:	00 
   25397:	48 8d 14 08          	lea    (%rax,%rcx,1),%rdx
   2539b:	48 c1 e2 03          	shl    $0x3,%rdx
   2539f:	48 8b b4 24 80 00 00 	mov    0x80(%rsp),%rsi
   253a6:	00 
   253a7:	48 69 c6 20 1c 00 00 	imul   $0x1c20,%rsi,%rax
   253ae:	48 03 84 24 b8 01 00 	add    0x1b8(%rsp),%rax
   253b5:	00 
   253b6:	4c 69 d6 00 e1 00 00 	imul   $0xe100,%rsi,%r10
   253bd:	4c 03 94 24 b0 01 00 	add    0x1b0(%rsp),%r10
   253c4:	00 
   253c5:	45 31 db             	xor    %r11d,%r11d
   253c8:	31 db                	xor    %ebx,%ebx
   253ca:	eb 20                	jmp    253ec <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x70c>
   253cc:	0f 1f 40 00          	nopl   0x0(%rax)
   253d0:	49 83 c3 08          	add    $0x8,%r11
   253d4:	48 ff c3             	inc    %rbx
   253d7:	48 83 c0 64          	add    $0x64,%rax
   253db:	49 81 c2 20 03 00 00 	add    $0x320,%r10
   253e2:	49 83 fb 40          	cmp    $0x40,%r11
   253e6:	0f 84 44 01 00 00    	je     25530 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x850>
   253ec:	f2 42 0f 10 b4 1c 20 	movsd  0x220(%rsp,%r11,1),%xmm6
   253f3:	02 00 00 
   253f6:	f2 0f 59 f5          	mulsd  %xmm5,%xmm6
   253fa:	f2 0f 5e 74 24 50    	divsd  0x50(%rsp),%xmm6
   25400:	66 0f 57 ff          	xorpd  %xmm7,%xmm7
   25404:	66 0f 2e fe          	ucomisd %xmm6,%xmm7
   25408:	73 c6                	jae    253d0 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x6f0>
   2540a:	4c 8d 34 13          	lea    (%rbx,%rdx,1),%r14
   2540e:	4b 8d 34 36          	lea    (%r14,%r14,1),%rsi
   25412:	49 c1 e6 05          	shl    $0x5,%r14
   25416:	49 29 f6             	sub    %rsi,%r14
   25419:	4c 89 d5             	mov    %r10,%rbp
   2541c:	49 89 c1             	mov    %rax,%r9
   2541f:	4c 8d a4 24 a0 00 00 	lea    0xa0(%rsp),%r12
   25426:	00 
   25427:	31 f6                	xor    %esi,%esi
   25429:	0f 1f 80 00 00 00 00 	nopl   0x0(%rax)
   25430:	4c 8d 7e 01          	lea    0x1(%rsi),%r15
   25434:	45 31 c0             	xor    %r8d,%r8d
   25437:	66 0f 1f 84 00 00 00 	nopw   0x0(%rax,%rax,1)
   2543e:	00 00 
   25440:	4b 8d 3c 01          	lea    (%r9,%r8,1),%rdi
   25444:	48 81 ff 7f 51 01 00 	cmp    $0x1517f,%rdi
   2544b:	0f 87 14 06 00 00    	ja     25a65 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0xd85>
   25451:	f2 0f 10 bc f4 a0 00 	movsd  0xa0(%rsp,%rsi,8),%xmm7
   25458:	00 00 
   2545a:	f2 0f 59 fe          	mulsd  %xmm6,%xmm7
   2545e:	f2 47 0f 10 04 c4    	movsd  (%r12,%r8,8),%xmm8
   25464:	f2 44 0f 59 c7       	mulsd  %xmm7,%xmm8
   25469:	f2 46 0f 58 44 c5 00 	addsd  0x0(%rbp,%r8,8),%xmm8
   25470:	f2 46 0f 11 44 c5 00 	movsd  %xmm8,0x0(%rbp,%r8,8)
   25477:	4a 8d 3c 06          	lea    (%rsi,%r8,1),%rdi
   2547b:	48 ff c7             	inc    %rdi
   2547e:	49 ff c0             	inc    %r8
   25481:	48 83 ff 0a          	cmp    $0xa,%rdi
   25485:	75 b9                	jne    25440 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x760>
   25487:	4c 8d 04 76          	lea    (%rsi,%rsi,2),%r8
   2548b:	4d 01 f0             	add    %r14,%r8
   2548e:	48 8b 74 24 20       	mov    0x20(%rsp),%rsi
   25493:	49 39 f0             	cmp    %rsi,%r8
   25496:	0f 83 07 10 00 00    	jae    264a3 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x17c3>
   2549c:	48 8b 74 24 18       	mov    0x18(%rsp),%rsi
   254a1:	66 44 0f 28 c0       	movapd %xmm0,%xmm8
   254a6:	f2 44 0f 59 c7       	mulsd  %xmm7,%xmm8
   254ab:	f2 46 0f 58 04 c6    	addsd  (%rsi,%r8,8),%xmm8
   254b1:	f2 46 0f 11 04 c6    	movsd  %xmm8,(%rsi,%r8,8)
   254b7:	49 8d 78 01          	lea    0x1(%r8),%rdi
   254bb:	48 8b 74 24 20       	mov    0x20(%rsp),%rsi
   254c0:	48 39 f7             	cmp    %rsi,%rdi
   254c3:	0f 83 ec 0f 00 00    	jae    264b5 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x17d5>
   254c9:	48 8b 74 24 18       	mov    0x18(%rsp),%rsi
   254ce:	66 44 0f 28 c1       	movapd %xmm1,%xmm8
   254d3:	f2 44 0f 59 c7       	mulsd  %xmm7,%xmm8
   254d8:	f2 46 0f 58 44 c6 08 	addsd  0x8(%rsi,%r8,8),%xmm8
   254df:	f2 46 0f 11 44 c6 08 	movsd  %xmm8,0x8(%rsi,%r8,8)
   254e6:	49 8d 78 02          	lea    0x2(%r8),%rdi
   254ea:	48 8b 74 24 20       	mov    0x20(%rsp),%rsi
   254ef:	48 39 f7             	cmp    %rsi,%rdi
   254f2:	0f 83 bd 0f 00 00    	jae    264b5 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x17d5>
   254f8:	48 8b 74 24 18       	mov    0x18(%rsp),%rsi
   254fd:	f2 0f 59 fa          	mulsd  %xmm2,%xmm7
   25501:	f2 42 0f 58 7c c6 10 	addsd  0x10(%rsi,%r8,8),%xmm7
   25508:	f2 42 0f 11 7c c6 10 	movsd  %xmm7,0x10(%rsi,%r8,8)
   2550f:	49 83 c4 08          	add    $0x8,%r12
   25513:	49 83 c1 0b          	add    $0xb,%r9
   25517:	48 83 c5 58          	add    $0x58,%rbp
   2551b:	4c 89 fe             	mov    %r15,%rsi
   2551e:	49 83 ff 0a          	cmp    $0xa,%r15
   25522:	0f 85 08 ff ff ff    	jne    25430 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x750>
   25528:	e9 a3 fe ff ff       	jmp    253d0 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x6f0>
   2552d:	0f 1f 00             	nopl   (%rax)
   25530:	48 8b 44 24 60       	mov    0x60(%rsp),%rax
   25535:	48 8d 14 c0          	lea    (%rax,%rax,8),%rdx
   25539:	66 0f 28 eb          	movapd %xmm3,%xmm5
   2553d:	f2 0f 59 ac 24 28 01 	mulsd  0x128(%rsp),%xmm5
   25544:	00 00 
   25546:	66 0f 57 f6          	xorpd  %xmm6,%xmm6
   2554a:	66 0f 2e f5          	ucomisd %xmm5,%xmm6
   2554e:	0f 83 9c 01 00 00    	jae    256f0 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0xa10>
   25554:	48 8b 84 24 c0 01 00 	mov    0x1c0(%rsp),%rax
   2555b:	00 
   2555c:	4c 8d 0c 10          	lea    (%rax,%rdx,1),%r9
   25560:	49 c1 e1 03          	shl    $0x3,%r9
   25564:	48 8b 44 24 60       	mov    0x60(%rsp),%rax
   25569:	4c 69 d0 20 1c 00 00 	imul   $0x1c20,%rax,%r10
   25570:	4c 03 94 24 b8 01 00 	add    0x1b8(%rsp),%r10
   25577:	00 
   25578:	4c 69 d8 00 e1 00 00 	imul   $0xe100,%rax,%r11
   2557f:	4c 03 9c 24 b0 01 00 	add    0x1b0(%rsp),%r11
   25586:	00 
   25587:	31 db                	xor    %ebx,%ebx
   25589:	45 31 f6             	xor    %r14d,%r14d
   2558c:	eb 1e                	jmp    255ac <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x8cc>
   2558e:	66 90                	xchg   %ax,%ax
   25590:	48 83 c3 08          	add    $0x8,%rbx
   25594:	49 ff c6             	inc    %r14
   25597:	49 83 c2 64          	add    $0x64,%r10
   2559b:	49 81 c3 20 03 00 00 	add    $0x320,%r11
   255a2:	48 83 fb 40          	cmp    $0x40,%rbx
   255a6:	0f 84 44 01 00 00    	je     256f0 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0xa10>
   255ac:	f2 0f 10 b4 1c 20 02 	movsd  0x220(%rsp,%rbx,1),%xmm6
   255b3:	00 00 
   255b5:	f2 0f 59 f5          	mulsd  %xmm5,%xmm6
   255b9:	f2 0f 5e 74 24 50    	divsd  0x50(%rsp),%xmm6
   255bf:	66 0f 57 ff          	xorpd  %xmm7,%xmm7
   255c3:	66 0f 2e fe          	ucomisd %xmm6,%xmm7
   255c7:	73 c7                	jae    25590 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x8b0>
   255c9:	4b 8d 2c 0e          	lea    (%r14,%r9,1),%rbp
   255cd:	48 8d 04 6d 00 00 00 	lea    0x0(,%rbp,2),%rax
   255d4:	00 
   255d5:	48 c1 e5 05          	shl    $0x5,%rbp
   255d9:	48 29 c5             	sub    %rax,%rbp
   255dc:	4c 89 d8             	mov    %r11,%rax
   255df:	4d 89 d4             	mov    %r10,%r12
   255e2:	4c 8d bc 24 a0 00 00 	lea    0xa0(%rsp),%r15
   255e9:	00 
   255ea:	31 f6                	xor    %esi,%esi
   255ec:	0f 1f 40 00          	nopl   0x0(%rax)
   255f0:	4c 8d 46 01          	lea    0x1(%rsi),%r8
   255f4:	45 31 ed             	xor    %r13d,%r13d
   255f7:	66 0f 1f 84 00 00 00 	nopw   0x0(%rax,%rax,1)
   255fe:	00 00 
   25600:	4b 8d 3c 2c          	lea    (%r12,%r13,1),%rdi
   25604:	48 81 ff 7f 51 01 00 	cmp    $0x1517f,%rdi
   2560b:	0f 87 54 04 00 00    	ja     25a65 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0xd85>
   25611:	f2 0f 10 bc f4 a0 00 	movsd  0xa0(%rsp,%rsi,8),%xmm7
   25618:	00 00 
   2561a:	f2 0f 59 fe          	mulsd  %xmm6,%xmm7
   2561e:	f2 47 0f 10 04 ef    	movsd  (%r15,%r13,8),%xmm8
   25624:	f2 44 0f 59 c7       	mulsd  %xmm7,%xmm8
   25629:	f2 46 0f 58 04 e8    	addsd  (%rax,%r13,8),%xmm8
   2562f:	f2 46 0f 11 04 e8    	movsd  %xmm8,(%rax,%r13,8)
   25635:	4a 8d 3c 2e          	lea    (%rsi,%r13,1),%rdi
   25639:	48 ff c7             	inc    %rdi
   2563c:	49 ff c5             	inc    %r13
   2563f:	48 83 ff 0a          	cmp    $0xa,%rdi
   25643:	75 bb                	jne    25600 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x920>
   25645:	4c 8d 2c 76          	lea    (%rsi,%rsi,2),%r13
   25649:	49 01 ed             	add    %rbp,%r13
   2564c:	48 8b 74 24 20       	mov    0x20(%rsp),%rsi
   25651:	49 39 f5             	cmp    %rsi,%r13
   25654:	0f 83 4e 0e 00 00    	jae    264a8 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x17c8>
   2565a:	48 8b 74 24 18       	mov    0x18(%rsp),%rsi
   2565f:	66 44 0f 28 c0       	movapd %xmm0,%xmm8
   25664:	f2 44 0f 59 c7       	mulsd  %xmm7,%xmm8
   25669:	f2 46 0f 58 04 ee    	addsd  (%rsi,%r13,8),%xmm8
   2566f:	f2 46 0f 11 04 ee    	movsd  %xmm8,(%rsi,%r13,8)
   25675:	49 8d 7d 01          	lea    0x1(%r13),%rdi
   25679:	48 8b 74 24 20       	mov    0x20(%rsp),%rsi
   2567e:	48 39 f7             	cmp    %rsi,%rdi
   25681:	0f 83 2e 0e 00 00    	jae    264b5 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x17d5>
   25687:	48 8b 74 24 18       	mov    0x18(%rsp),%rsi
   2568c:	66 44 0f 28 c1       	movapd %xmm1,%xmm8
   25691:	f2 44 0f 59 c7       	mulsd  %xmm7,%xmm8
   25696:	f2 46 0f 58 44 ee 08 	addsd  0x8(%rsi,%r13,8),%xmm8
   2569d:	f2 46 0f 11 44 ee 08 	movsd  %xmm8,0x8(%rsi,%r13,8)
   256a4:	49 8d 7d 02          	lea    0x2(%r13),%rdi
   256a8:	48 8b 74 24 20       	mov    0x20(%rsp),%rsi
   256ad:	48 39 f7             	cmp    %rsi,%rdi
   256b0:	0f 83 ff 0d 00 00    	jae    264b5 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x17d5>
   256b6:	48 8b 74 24 18       	mov    0x18(%rsp),%rsi
   256bb:	f2 0f 59 fa          	mulsd  %xmm2,%xmm7
   256bf:	f2 42 0f 58 7c ee 10 	addsd  0x10(%rsi,%r13,8),%xmm7
   256c6:	f2 42 0f 11 7c ee 10 	movsd  %xmm7,0x10(%rsi,%r13,8)
   256cd:	49 83 c7 08          	add    $0x8,%r15
   256d1:	49 83 c4 0b          	add    $0xb,%r12
   256d5:	48 83 c0 58          	add    $0x58,%rax
   256d9:	4c 89 c6             	mov    %r8,%rsi
   256dc:	49 83 f8 0a          	cmp    $0xa,%r8
   256e0:	0f 85 0a ff ff ff    	jne    255f0 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x910>
   256e6:	e9 a5 fe ff ff       	jmp    25590 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x8b0>
   256eb:	0f 1f 44 00 00       	nopl   0x0(%rax,%rax,1)
   256f0:	f2 0f 59 a4 24 c8 01 	mulsd  0x1c8(%rsp),%xmm4
   256f7:	00 00 
   256f9:	66 0f 57 ed          	xorpd  %xmm5,%xmm5
   256fd:	66 0f 2e ec          	ucomisd %xmm4,%xmm5
   25701:	4c 8d ac 24 f0 00 00 	lea    0xf0(%rsp),%r13
   25708:	00 
   25709:	48 8b 6c 24 08       	mov    0x8(%rsp),%rbp
   2570e:	0f 83 9c 01 00 00    	jae    258b0 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0xbd0>
   25714:	48 03 8c 24 a8 01 00 	add    0x1a8(%rsp),%rcx
   2571b:	00 
   2571c:	48 c1 e1 03          	shl    $0x3,%rcx
   25720:	48 8b 84 24 80 00 00 	mov    0x80(%rsp),%rax
   25727:	00 
   25728:	4c 69 c8 20 1c 00 00 	imul   $0x1c20,%rax,%r9
   2572f:	4c 03 8c 24 a0 01 00 	add    0x1a0(%rsp),%r9
   25736:	00 
   25737:	4c 69 c0 00 e1 00 00 	imul   $0xe100,%rax,%r8
   2573e:	4c 03 84 24 98 01 00 	add    0x198(%rsp),%r8
   25745:	00 
   25746:	45 31 d2             	xor    %r10d,%r10d
   25749:	45 31 db             	xor    %r11d,%r11d
   2574c:	eb 1e                	jmp    2576c <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0xa8c>
   2574e:	66 90                	xchg   %ax,%ax
   25750:	49 83 c2 08          	add    $0x8,%r10
   25754:	49 ff c3             	inc    %r11
   25757:	49 83 c1 64          	add    $0x64,%r9
   2575b:	49 81 c0 20 03 00 00 	add    $0x320,%r8
   25762:	49 83 fa 40          	cmp    $0x40,%r10
   25766:	0f 84 44 01 00 00    	je     258b0 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0xbd0>
   2576c:	f2 42 0f 10 ac 14 20 	movsd  0x220(%rsp,%r10,1),%xmm5
   25773:	02 00 00 
   25776:	f2 0f 59 ec          	mulsd  %xmm4,%xmm5
   2577a:	f2 0f 5e 6c 24 50    	divsd  0x50(%rsp),%xmm5
   25780:	66 0f 57 f6          	xorpd  %xmm6,%xmm6
   25784:	66 0f 2e f5          	ucomisd %xmm5,%xmm6
   25788:	73 c6                	jae    25750 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0xa70>
   2578a:	49 8d 1c 0b          	lea    (%r11,%rcx,1),%rbx
   2578e:	48 8d 04 1b          	lea    (%rbx,%rbx,1),%rax
   25792:	48 c1 e3 05          	shl    $0x5,%rbx
   25796:	48 29 c3             	sub    %rax,%rbx
   25799:	4c 89 c0             	mov    %r8,%rax
   2579c:	4d 89 ce             	mov    %r9,%r14
   2579f:	4c 8d a4 24 a0 00 00 	lea    0xa0(%rsp),%r12
   257a6:	00 
   257a7:	31 f6                	xor    %esi,%esi
   257a9:	0f 1f 80 00 00 00 00 	nopl   0x0(%rax)
   257b0:	4c 8d 7e 01          	lea    0x1(%rsi),%r15
   257b4:	45 31 ed             	xor    %r13d,%r13d
   257b7:	66 0f 1f 84 00 00 00 	nopw   0x0(%rax,%rax,1)
   257be:	00 00 
   257c0:	4b 8d 3c 2e          	lea    (%r14,%r13,1),%rdi
   257c4:	48 81 ff 7f 51 01 00 	cmp    $0x1517f,%rdi
   257cb:	0f 87 94 02 00 00    	ja     25a65 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0xd85>
   257d1:	f2 0f 10 b4 f4 a0 00 	movsd  0xa0(%rsp,%rsi,8),%xmm6
   257d8:	00 00 
   257da:	f2 0f 59 f5          	mulsd  %xmm5,%xmm6
   257de:	f2 43 0f 10 3c ec    	movsd  (%r12,%r13,8),%xmm7
   257e4:	f2 0f 59 fe          	mulsd  %xmm6,%xmm7
   257e8:	f2 42 0f 58 3c e8    	addsd  (%rax,%r13,8),%xmm7
   257ee:	f2 42 0f 11 3c e8    	movsd  %xmm7,(%rax,%r13,8)
   257f4:	4a 8d 3c 2e          	lea    (%rsi,%r13,1),%rdi
   257f8:	48 ff c7             	inc    %rdi
   257fb:	49 ff c5             	inc    %r13
   257fe:	48 83 ff 0a          	cmp    $0xa,%rdi
   25802:	75 bc                	jne    257c0 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0xae0>
   25804:	48 8d 2c 76          	lea    (%rsi,%rsi,2),%rbp
   25808:	48 01 dd             	add    %rbx,%rbp
   2580b:	48 8b 74 24 20       	mov    0x20(%rsp),%rsi
   25810:	48 39 f5             	cmp    %rsi,%rbp
   25813:	0f 83 94 0c 00 00    	jae    264ad <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x17cd>
   25819:	48 8b 74 24 18       	mov    0x18(%rsp),%rsi
   2581e:	66 0f 28 f8          	movapd %xmm0,%xmm7
   25822:	f2 0f 59 fe          	mulsd  %xmm6,%xmm7
   25826:	f2 0f 58 3c ee       	addsd  (%rsi,%rbp,8),%xmm7
   2582b:	f2 0f 11 3c ee       	movsd  %xmm7,(%rsi,%rbp,8)
   25830:	48 8d 7d 01          	lea    0x1(%rbp),%rdi
   25834:	48 8b 74 24 20       	mov    0x20(%rsp),%rsi
   25839:	48 39 f7             	cmp    %rsi,%rdi
   2583c:	0f 83 73 0c 00 00    	jae    264b5 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x17d5>
   25842:	48 8b 74 24 18       	mov    0x18(%rsp),%rsi
   25847:	66 0f 28 f9          	movapd %xmm1,%xmm7
   2584b:	f2 0f 59 fe          	mulsd  %xmm6,%xmm7
   2584f:	f2 0f 58 7c ee 08    	addsd  0x8(%rsi,%rbp,8),%xmm7
   25855:	f2 0f 11 7c ee 08    	movsd  %xmm7,0x8(%rsi,%rbp,8)
   2585b:	48 8d 7d 02          	lea    0x2(%rbp),%rdi
   2585f:	48 8b 74 24 20       	mov    0x20(%rsp),%rsi
   25864:	48 39 f7             	cmp    %rsi,%rdi
   25867:	0f 83 48 0c 00 00    	jae    264b5 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x17d5>
   2586d:	48 8b 74 24 18       	mov    0x18(%rsp),%rsi
   25872:	f2 0f 59 f2          	mulsd  %xmm2,%xmm6
   25876:	f2 0f 58 74 ee 10    	addsd  0x10(%rsi,%rbp,8),%xmm6
   2587c:	f2 0f 11 74 ee 10    	movsd  %xmm6,0x10(%rsi,%rbp,8)
   25882:	49 83 c4 08          	add    $0x8,%r12
   25886:	49 83 c6 0b          	add    $0xb,%r14
   2588a:	48 83 c0 58          	add    $0x58,%rax
   2588e:	4c 89 fe             	mov    %r15,%rsi
   25891:	49 83 ff 0a          	cmp    $0xa,%r15
   25895:	4c 8d ac 24 f0 00 00 	lea    0xf0(%rsp),%r13
   2589c:	00 
   2589d:	48 8b 6c 24 08       	mov    0x8(%rsp),%rbp
   258a2:	0f 85 08 ff ff ff    	jne    257b0 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0xad0>
   258a8:	e9 a3 fe ff ff       	jmp    25750 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0xa70>
   258ad:	0f 1f 00             	nopl   (%rax)
   258b0:	f2 0f 59 9c 24 c8 01 	mulsd  0x1c8(%rsp),%xmm3
   258b7:	00 00 
   258b9:	66 0f 57 e4          	xorpd  %xmm4,%xmm4
   258bd:	66 0f 2e e3          	ucomisd %xmm3,%xmm4
   258c1:	0f 83 79 f6 ff ff    	jae    24f40 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x260>
   258c7:	48 03 94 24 a8 01 00 	add    0x1a8(%rsp),%rdx
   258ce:	00 
   258cf:	48 c1 e2 03          	shl    $0x3,%rdx
   258d3:	48 8b 44 24 60       	mov    0x60(%rsp),%rax
   258d8:	48 69 c8 20 1c 00 00 	imul   $0x1c20,%rax,%rcx
   258df:	48 03 8c 24 a0 01 00 	add    0x1a0(%rsp),%rcx
   258e6:	00 
   258e7:	48 69 c0 00 e1 00 00 	imul   $0xe100,%rax,%rax
   258ee:	48 03 84 24 98 01 00 	add    0x198(%rsp),%rax
   258f5:	00 
   258f6:	45 31 c0             	xor    %r8d,%r8d
   258f9:	45 31 c9             	xor    %r9d,%r9d
   258fc:	eb 1d                	jmp    2591b <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0xc3b>
   258fe:	66 90                	xchg   %ax,%ax
   25900:	49 83 c0 08          	add    $0x8,%r8
   25904:	49 ff c1             	inc    %r9
   25907:	48 83 c1 64          	add    $0x64,%rcx
   2590b:	48 05 20 03 00 00    	add    $0x320,%rax
   25911:	49 83 f8 40          	cmp    $0x40,%r8
   25915:	0f 84 25 f6 ff ff    	je     24f40 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x260>
   2591b:	f2 42 0f 10 a4 04 20 	movsd  0x220(%rsp,%r8,1),%xmm4
   25922:	02 00 00 
   25925:	f2 0f 59 e3          	mulsd  %xmm3,%xmm4
   25929:	f2 0f 5e 64 24 50    	divsd  0x50(%rsp),%xmm4
   2592f:	66 0f 57 ed          	xorpd  %xmm5,%xmm5
   25933:	66 0f 2e ec          	ucomisd %xmm4,%xmm5
   25937:	73 c7                	jae    25900 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0xc20>
   25939:	4d 8d 14 11          	lea    (%r9,%rdx,1),%r10
   2593d:	4b 8d 34 12          	lea    (%r10,%r10,1),%rsi
   25941:	49 c1 e2 05          	shl    $0x5,%r10
   25945:	49 29 f2             	sub    %rsi,%r10
   25948:	49 89 c3             	mov    %rax,%r11
   2594b:	48 89 cb             	mov    %rcx,%rbx
   2594e:	4c 8d b4 24 a0 00 00 	lea    0xa0(%rsp),%r14
   25955:	00 
   25956:	31 f6                	xor    %esi,%esi
   25958:	0f 1f 84 00 00 00 00 	nopl   0x0(%rax,%rax,1)
   2595f:	00 
   25960:	4c 8d 7e 01          	lea    0x1(%rsi),%r15
   25964:	45 31 e4             	xor    %r12d,%r12d
   25967:	66 0f 1f 84 00 00 00 	nopw   0x0(%rax,%rax,1)
   2596e:	00 00 
   25970:	4a 8d 3c 23          	lea    (%rbx,%r12,1),%rdi
   25974:	48 81 ff 7f 51 01 00 	cmp    $0x1517f,%rdi
   2597b:	0f 87 e4 00 00 00    	ja     25a65 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0xd85>
   25981:	f2 0f 10 ac f4 a0 00 	movsd  0xa0(%rsp,%rsi,8),%xmm5
   25988:	00 00 
   2598a:	f2 0f 59 ec          	mulsd  %xmm4,%xmm5
   2598e:	f2 43 0f 10 34 e6    	movsd  (%r14,%r12,8),%xmm6
   25994:	f2 0f 59 f5          	mulsd  %xmm5,%xmm6
   25998:	f2 43 0f 58 34 e3    	addsd  (%r11,%r12,8),%xmm6
   2599e:	f2 43 0f 11 34 e3    	movsd  %xmm6,(%r11,%r12,8)
   259a4:	4a 8d 3c 26          	lea    (%rsi,%r12,1),%rdi
   259a8:	48 ff c7             	inc    %rdi
   259ab:	49 ff c4             	inc    %r12
   259ae:	48 83 ff 0a          	cmp    $0xa,%rdi
   259b2:	75 bc                	jne    25970 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0xc90>
   259b4:	4c 8d 24 76          	lea    (%rsi,%rsi,2),%r12
   259b8:	4d 01 d4             	add    %r10,%r12
   259bb:	48 8b 74 24 20       	mov    0x20(%rsp),%rsi
   259c0:	49 39 f4             	cmp    %rsi,%r12
   259c3:	0f 83 e9 0a 00 00    	jae    264b2 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x17d2>
   259c9:	48 8b 74 24 18       	mov    0x18(%rsp),%rsi
   259ce:	66 0f 28 f0          	movapd %xmm0,%xmm6
   259d2:	f2 0f 59 f5          	mulsd  %xmm5,%xmm6
   259d6:	f2 42 0f 58 34 e6    	addsd  (%rsi,%r12,8),%xmm6
   259dc:	f2 42 0f 11 34 e6    	movsd  %xmm6,(%rsi,%r12,8)
   259e2:	49 8d 7c 24 01       	lea    0x1(%r12),%rdi
   259e7:	48 8b 74 24 20       	mov    0x20(%rsp),%rsi
   259ec:	48 39 f7             	cmp    %rsi,%rdi
   259ef:	0f 83 c0 0a 00 00    	jae    264b5 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x17d5>
   259f5:	48 8b 74 24 18       	mov    0x18(%rsp),%rsi
   259fa:	66 0f 28 f1          	movapd %xmm1,%xmm6
   259fe:	f2 0f 59 f5          	mulsd  %xmm5,%xmm6
   25a02:	f2 42 0f 58 74 e6 08 	addsd  0x8(%rsi,%r12,8),%xmm6
   25a09:	f2 42 0f 11 74 e6 08 	movsd  %xmm6,0x8(%rsi,%r12,8)
   25a10:	49 8d 7c 24 02       	lea    0x2(%r12),%rdi
   25a15:	48 8b 74 24 20       	mov    0x20(%rsp),%rsi
   25a1a:	48 39 f7             	cmp    %rsi,%rdi
   25a1d:	0f 83 92 0a 00 00    	jae    264b5 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x17d5>
   25a23:	48 8b 74 24 18       	mov    0x18(%rsp),%rsi
   25a28:	f2 0f 59 ea          	mulsd  %xmm2,%xmm5
   25a2c:	f2 42 0f 58 6c e6 10 	addsd  0x10(%rsi,%r12,8),%xmm5
   25a33:	f2 42 0f 11 6c e6 10 	movsd  %xmm5,0x10(%rsi,%r12,8)
   25a3a:	49 83 c6 08          	add    $0x8,%r14
   25a3e:	48 83 c3 0b          	add    $0xb,%rbx
   25a42:	49 83 c3 58          	add    $0x58,%r11
   25a46:	4c 89 fe             	mov    %r15,%rsi
   25a49:	49 83 ff 0a          	cmp    $0xa,%r15
   25a4d:	4c 8d ac 24 f0 00 00 	lea    0xf0(%rsp),%r13
   25a54:	00 
   25a55:	48 8b 6c 24 08       	mov    0x8(%rsp),%rbp
   25a5a:	0f 85 00 ff ff ff    	jne    25960 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0xc80>
   25a60:	e9 9b fe ff ff       	jmp    25900 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0xc20>
   25a65:	48 8d 15 4c 79 04 00 	lea    0x4794c(%rip),%rdx        # 6d3b8 <anon.441e142b17a370d935234e81e4cac821.6.llvm.308828945136828694+0x228>
   25a6c:	bb 80 51 01 00       	mov    $0x15180,%ebx
   25a71:	48 89 de             	mov    %rbx,%rsi
   25a74:	ff 15 ee 9f 04 00    	call   *0x49fee(%rip)        # 6fa68 <_DYNAMIC+0x3f8>
   25a7a:	e9 4b 12 00 00       	jmp    26cca <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1fea>
   25a7f:	48 8d bc 24 f0 00 00 	lea    0xf0(%rsp),%rdi
   25a86:	00 
   25a87:	be 40 65 00 00       	mov    $0x6540,%esi
   25a8c:	e8 af df ff ff       	call   23a40 <<f64 as alloc::vec::spec_from_elem::SpecFromElem>::from_elem::<alloc::alloc::Global>>
   25a91:	b8 80 51 01 00       	mov    $0x15180,%eax
   25a96:	31 c9                	xor    %ecx,%ecx
   25a98:	31 d2                	xor    %edx,%edx
   25a9a:	48 8b 74 24 78       	mov    0x78(%rsp),%rsi
   25a9f:	31 ff                	xor    %edi,%edi
   25aa1:	4c 8d 04 ff          	lea    (%rdi,%rdi,8),%r8
   25aa5:	4c 89 84 24 e0 01 00 	mov    %r8,0x1e0(%rsp)
   25aac:	00 
   25aad:	48 ff c7             	inc    %rdi
   25ab0:	48 89 bc 24 28 01 00 	mov    %rdi,0x128(%rsp)
   25ab7:	00 
   25ab8:	48 89 8c 24 58 01 00 	mov    %rcx,0x158(%rsp)
   25abf:	00 
   25ac0:	48 89 94 24 50 01 00 	mov    %rdx,0x150(%rsp)
   25ac7:	00 
   25ac8:	48 89 b4 24 48 01 00 	mov    %rsi,0x148(%rsp)
   25acf:	00 
   25ad0:	48 89 84 24 d0 01 00 	mov    %rax,0x1d0(%rsp)
   25ad7:	00 
   25ad8:	31 ff                	xor    %edi,%edi
   25ada:	eb 4d                	jmp    25b29 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0xe49>
   25adc:	48 8b 84 24 90 00 00 	mov    0x90(%rsp),%rax
   25ae3:	00 
   25ae4:	48 05 e0 fc ff ff    	add    $0xfffffffffffffce0,%rax
   25aea:	48 8b b4 24 60 01 00 	mov    0x160(%rsp),%rsi
   25af1:	00 
   25af2:	48 81 c6 00 19 00 00 	add    $0x1900,%rsi
   25af9:	48 8b 94 24 30 01 00 	mov    0x130(%rsp),%rdx
   25b00:	00 
   25b01:	48 81 c2 20 03 00 00 	add    $0x320,%rdx
   25b08:	48 8b 8c 24 70 01 00 	mov    0x170(%rsp),%rcx
   25b0f:	00 
   25b10:	48 81 c1 f0 00 00 00 	add    $0xf0,%rcx
   25b17:	48 8b bc 24 f0 01 00 	mov    0x1f0(%rsp),%rdi
   25b1e:	00 
   25b1f:	48 83 ff 09          	cmp    $0x9,%rdi
   25b23:	0f 84 88 08 00 00    	je     263b1 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x16d1>
   25b29:	49 89 f8             	mov    %rdi,%r8
   25b2c:	48 ff c7             	inc    %rdi
   25b2f:	48 89 bc 24 f0 01 00 	mov    %rdi,0x1f0(%rsp)
   25b36:	00 
   25b37:	4c 03 84 24 e0 01 00 	add    0x1e0(%rsp),%r8
   25b3e:	00 
   25b3f:	49 c1 e0 03          	shl    $0x3,%r8
   25b43:	4c 89 84 24 80 01 00 	mov    %r8,0x180(%rsp)
   25b4a:	00 
   25b4b:	48 89 8c 24 70 01 00 	mov    %rcx,0x170(%rsp)
   25b52:	00 
   25b53:	49 89 cc             	mov    %rcx,%r12
   25b56:	48 89 94 24 30 01 00 	mov    %rdx,0x130(%rsp)
   25b5d:	00 
   25b5e:	48 89 d3             	mov    %rdx,%rbx
   25b61:	48 89 b4 24 60 01 00 	mov    %rsi,0x160(%rsp)
   25b68:	00 
   25b69:	48 89 74 24 50       	mov    %rsi,0x50(%rsp)
   25b6e:	48 89 84 24 90 00 00 	mov    %rax,0x90(%rsp)
   25b75:	00 
   25b76:	48 89 44 24 60       	mov    %rax,0x60(%rsp)
   25b7b:	31 ed                	xor    %ebp,%ebp
   25b7d:	eb 41                	jmp    25bc0 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0xee0>
   25b7f:	90                   	nop
   25b80:	be 20 03 00 00       	mov    $0x320,%esi
   25b85:	ba 08 00 00 00       	mov    $0x8,%edx
   25b8a:	4c 89 f7             	mov    %r14,%rdi
   25b8d:	ff d3                	call   *%rbx
   25b8f:	48 83 44 24 60 9c    	addq   $0xffffffffffffff9c,0x60(%rsp)
   25b95:	48 81 44 24 50 20 03 	addq   $0x320,0x50(%rsp)
   25b9c:	00 00 
   25b9e:	48 8b 9c 24 10 01 00 	mov    0x110(%rsp),%rbx
   25ba5:	00 
   25ba6:	48 83 c3 64          	add    $0x64,%rbx
   25baa:	49 83 c4 1e          	add    $0x1e,%r12
   25bae:	48 8b 44 24 70       	mov    0x70(%rsp),%rax
   25bb3:	48 89 c5             	mov    %rax,%rbp
   25bb6:	48 83 f8 08          	cmp    $0x8,%rax
   25bba:	0f 84 1c ff ff ff    	je     25adc <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0xdfc>
   25bc0:	ff 15 62 9d 04 00    	call   *0x49d62(%rip)        # 6f928 <_DYNAMIC+0x2b8>
   25bc6:	41 bf 20 03 00 00    	mov    $0x320,%r15d
   25bcc:	41 bd 08 00 00 00    	mov    $0x8,%r13d
   25bd2:	bf 20 03 00 00       	mov    $0x320,%edi
   25bd7:	be 08 00 00 00       	mov    $0x8,%esi
   25bdc:	ff 15 66 9e 04 00    	call   *0x49e66(%rip)        # 6fa48 <_DYNAMIC+0x3d8>
   25be2:	48 85 c0             	test   %rax,%rax
   25be5:	0f 84 b1 10 00 00    	je     26c9c <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1fbc>
   25beb:	49 89 c6             	mov    %rax,%r14
   25bee:	48 8d 45 01          	lea    0x1(%rbp),%rax
   25bf2:	48 89 44 24 70       	mov    %rax,0x70(%rsp)
   25bf7:	48 03 ac 24 80 01 00 	add    0x180(%rsp),%rbp
   25bfe:	00 
   25bff:	48 89 ac 24 80 00 00 	mov    %rbp,0x80(%rsp)
   25c06:	00 
   25c07:	49 8d 46 18          	lea    0x18(%r14),%rax
   25c0b:	b9 64 00 00 00       	mov    $0x64,%ecx
   25c10:	48 89 9c 24 10 01 00 	mov    %rbx,0x110(%rsp)
   25c17:	00 
   25c18:	48 89 da             	mov    %rbx,%rdx
   25c1b:	48 8b 74 24 60       	mov    0x60(%rsp),%rsi
   25c20:	4d 89 f0             	mov    %r14,%r8
   25c23:	4c 8b 4c 24 50       	mov    0x50(%rsp),%r9
   25c28:	45 31 d2             	xor    %r10d,%r10d
   25c2b:	4c 89 e3             	mov    %r12,%rbx
   25c2e:	4c 89 74 24 08       	mov    %r14,0x8(%rsp)
   25c33:	66 66 66 66 2e 0f 1f 	data16 data16 data16 cs nopw 0x0(%rax,%rax,1)
   25c3a:	84 00 00 00 00 00 
   25c40:	4d 85 d2             	test   %r10,%r10
   25c43:	74 1b                	je     25c60 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0xf80>
   25c45:	4b 8d 3c 12          	lea    (%r10,%r10,1),%rdi
   25c49:	48 8d 3c bf          	lea    (%rdi,%rdi,4),%rdi
   25c4d:	49 83 fa 04          	cmp    $0x4,%r10
   25c51:	73 1d                	jae    25c70 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0xf90>
   25c53:	45 31 db             	xor    %r11d,%r11d
   25c56:	eb 77                	jmp    25ccf <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0xfef>
   25c58:	0f 1f 84 00 00 00 00 	nopl   0x0(%rax,%rax,1)
   25c5f:	00 
   25c60:	31 ff                	xor    %edi,%edi
   25c62:	e9 a1 00 00 00       	jmp    25d08 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1028>
   25c67:	66 0f 1f 84 00 00 00 	nopw   0x0(%rax,%rax,1)
   25c6e:	00 00 
   25c70:	4d 89 d7             	mov    %r10,%r15
   25c73:	49 83 e7 fc          	and    $0xfffffffffffffffc,%r15
   25c77:	4d 89 cd             	mov    %r9,%r13
   25c7a:	45 31 db             	xor    %r11d,%r11d
   25c7d:	0f 1f 00             	nopl   (%rax)
   25c80:	f2 41 0f 10 45 00    	movsd  0x0(%r13),%xmm0
   25c86:	f2 42 0f 11 44 d8 e8 	movsd  %xmm0,-0x18(%rax,%r11,8)
   25c8d:	f2 41 0f 10 45 50    	movsd  0x50(%r13),%xmm0
   25c93:	f2 42 0f 11 44 d8 f0 	movsd  %xmm0,-0x10(%rax,%r11,8)
   25c9a:	f2 41 0f 10 85 a0 00 	movsd  0xa0(%r13),%xmm0
   25ca1:	00 00 
   25ca3:	f2 42 0f 11 44 d8 f8 	movsd  %xmm0,-0x8(%rax,%r11,8)
   25caa:	f2 41 0f 10 85 f0 00 	movsd  0xf0(%r13),%xmm0
   25cb1:	00 00 
   25cb3:	f2 42 0f 11 04 d8    	movsd  %xmm0,(%rax,%r11,8)
   25cb9:	49 83 c3 04          	add    $0x4,%r11
   25cbd:	49 81 c5 40 01 00 00 	add    $0x140,%r13
   25cc4:	4d 39 df             	cmp    %r11,%r15
   25cc7:	75 b7                	jne    25c80 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0xfa0>
   25cc9:	41 f6 c2 03          	test   $0x3,%r10b
   25ccd:	74 39                	je     25d08 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1028>
   25ccf:	45 89 d7             	mov    %r10d,%r15d
   25cd2:	41 83 e7 03          	and    $0x3,%r15d
   25cd6:	4f 8d 2c d8          	lea    (%r8,%r11,8),%r13
   25cda:	4f 8d 1c 9b          	lea    (%r11,%r11,4),%r11
   25cde:	49 c1 e3 04          	shl    $0x4,%r11
   25ce2:	4d 01 cb             	add    %r9,%r11
   25ce5:	45 31 e4             	xor    %r12d,%r12d
   25ce8:	0f 1f 84 00 00 00 00 	nopl   0x0(%rax,%rax,1)
   25cef:	00 
   25cf0:	f2 41 0f 10 03       	movsd  (%r11),%xmm0
   25cf5:	f2 43 0f 11 44 e5 00 	movsd  %xmm0,0x0(%r13,%r12,8)
   25cfc:	49 ff c4             	inc    %r12
   25cff:	49 83 c3 50          	add    $0x50,%r11
   25d03:	4d 39 e7             	cmp    %r12,%r15
   25d06:	75 e8                	jne    25cf0 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1010>
   25d08:	4d 8d 5a 01          	lea    0x1(%r10),%r11
   25d0c:	4e 8d 3c 17          	lea    (%rdi,%r10,1),%r15
   25d10:	4f 8d 2c fe          	lea    (%r14,%r15,8),%r13
   25d14:	48 01 d7             	add    %rdx,%rdi
   25d17:	4c 8b 74 24 78       	mov    0x78(%rsp),%r14
   25d1c:	4d 8d 24 fe          	lea    (%r14,%rdi,8),%r12
   25d20:	31 ed                	xor    %ebp,%ebp
   25d22:	66 66 66 66 66 2e 0f 	data16 data16 data16 data16 cs nopw 0x0(%rax,%rax,1)
   25d29:	1f 84 00 00 00 00 00 
   25d30:	48 39 ee             	cmp    %rbp,%rsi
   25d33:	0f 84 37 07 00 00    	je     26470 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1790>
   25d39:	48 39 e9             	cmp    %rbp,%rcx
   25d3c:	0f 84 3f 07 00 00    	je     26481 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x17a1>
   25d42:	f2 41 0f 10 04 ec    	movsd  (%r12,%rbp,8),%xmm0
   25d48:	f2 41 0f 11 44 ed 00 	movsd  %xmm0,0x0(%r13,%rbp,8)
   25d4f:	4d 8d 34 2a          	lea    (%r10,%rbp,1),%r14
   25d53:	49 ff c6             	inc    %r14
   25d56:	48 ff c5             	inc    %rbp
   25d59:	49 83 fe 0a          	cmp    $0xa,%r14
   25d5d:	75 d1                	jne    25d30 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1050>
   25d5f:	49 83 c1 08          	add    $0x8,%r9
   25d63:	48 83 c0 50          	add    $0x50,%rax
   25d67:	49 83 c0 50          	add    $0x50,%r8
   25d6b:	48 83 c1 f5          	add    $0xfffffffffffffff5,%rcx
   25d6f:	48 83 c6 f5          	add    $0xfffffffffffffff5,%rsi
   25d73:	48 ff c2             	inc    %rdx
   25d76:	4d 89 da             	mov    %r11,%r10
   25d79:	49 83 fb 0a          	cmp    $0xa,%r11
   25d7d:	4c 8b 74 24 08       	mov    0x8(%rsp),%r14
   25d82:	0f 85 b8 fe ff ff    	jne    25c40 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0xf60>
   25d88:	49 89 dc             	mov    %rbx,%r12
   25d8b:	f2 45 0f 10 16       	movsd  (%r14),%xmm10
   25d90:	f2 41 0f 10 7e 58    	movsd  0x58(%r14),%xmm7
   25d96:	66 41 0f 28 c2       	movapd %xmm10,%xmm0
   25d9b:	f2 0f 58 c7          	addsd  %xmm7,%xmm0
   25d9f:	f2 45 0f 10 86 b0 00 	movsd  0xb0(%r14),%xmm8
   25da6:	00 00 
   25da8:	f2 41 0f 58 c0       	addsd  %xmm8,%xmm0
   25dad:	f2 45 0f 10 8e 08 01 	movsd  0x108(%r14),%xmm9
   25db4:	00 00 
   25db6:	f2 41 0f 58 c1       	addsd  %xmm9,%xmm0
   25dbb:	f2 41 0f 10 8e 60 01 	movsd  0x160(%r14),%xmm1
   25dc2:	00 00 
   25dc4:	f2 0f 58 c1          	addsd  %xmm1,%xmm0
   25dc8:	f2 41 0f 10 96 b8 01 	movsd  0x1b8(%r14),%xmm2
   25dcf:	00 00 
   25dd1:	f2 0f 58 c2          	addsd  %xmm2,%xmm0
   25dd5:	f2 41 0f 10 9e 10 02 	movsd  0x210(%r14),%xmm3
   25ddc:	00 00 
   25dde:	f2 0f 58 c3          	addsd  %xmm3,%xmm0
   25de2:	f2 41 0f 10 a6 68 02 	movsd  0x268(%r14),%xmm4
   25de9:	00 00 
   25deb:	f2 0f 58 c4          	addsd  %xmm4,%xmm0
   25def:	f2 41 0f 10 ae c0 02 	movsd  0x2c0(%r14),%xmm5
   25df6:	00 00 
   25df8:	f2 0f 58 c5          	addsd  %xmm5,%xmm0
   25dfc:	f2 41 0f 10 b6 18 03 	movsd  0x318(%r14),%xmm6
   25e03:	00 00 
   25e05:	f2 0f 58 c6          	addsd  %xmm6,%xmm0
   25e09:	f2 44 0f 10 1d d6 6f 	movsd  -0x1902a(%rip),%xmm11        # cde8 <GCC_except_table0+0x700>
   25e10:	fe ff 
   25e12:	f2 41 0f 5e c3       	divsd  %xmm11,%xmm0
   25e17:	f2 0f 59 05 f9 6f fe 	mulsd  -0x19007(%rip),%xmm0        # ce18 <GCC_except_table0+0x730>
   25e1e:	ff 
   25e1f:	f2 0f 5f 05 d9 6f fe 	maxsd  -0x19027(%rip),%xmm0        # ce00 <GCC_except_table0+0x718>
   25e26:	ff 
   25e27:	f2 44 0f 58 d0       	addsd  %xmm0,%xmm10
   25e2c:	f2 45 0f 11 16       	movsd  %xmm10,(%r14)
   25e31:	f2 0f 58 f8          	addsd  %xmm0,%xmm7
   25e35:	f2 41 0f 11 7e 58    	movsd  %xmm7,0x58(%r14)
   25e3b:	f2 44 0f 58 c0       	addsd  %xmm0,%xmm8
   25e40:	f2 45 0f 11 86 b0 00 	movsd  %xmm8,0xb0(%r14)
   25e47:	00 00 
   25e49:	f2 44 0f 58 c8       	addsd  %xmm0,%xmm9
   25e4e:	f2 45 0f 11 8e 08 01 	movsd  %xmm9,0x108(%r14)
   25e55:	00 00 
   25e57:	f2 41 0f 59 c3       	mulsd  %xmm11,%xmm0
   25e5c:	f2 0f 58 c8          	addsd  %xmm0,%xmm1
   25e60:	f2 41 0f 11 8e 60 01 	movsd  %xmm1,0x160(%r14)
   25e67:	00 00 
   25e69:	f2 0f 58 d0          	addsd  %xmm0,%xmm2
   25e6d:	f2 41 0f 11 96 b8 01 	movsd  %xmm2,0x1b8(%r14)
   25e74:	00 00 
   25e76:	f2 0f 58 d8          	addsd  %xmm0,%xmm3
   25e7a:	f2 41 0f 11 9e 10 02 	movsd  %xmm3,0x210(%r14)
   25e81:	00 00 
   25e83:	f2 0f 58 e0          	addsd  %xmm0,%xmm4
   25e87:	f2 41 0f 11 a6 68 02 	movsd  %xmm4,0x268(%r14)
   25e8e:	00 00 
   25e90:	f2 0f 58 e8          	addsd  %xmm0,%xmm5
   25e94:	f2 41 0f 11 ae c0 02 	movsd  %xmm5,0x2c0(%r14)
   25e9b:	00 00 
   25e9d:	f2 0f 58 c6          	addsd  %xmm6,%xmm0
   25ea1:	f2 41 0f 11 86 18 03 	movsd  %xmm0,0x318(%r14)
   25ea8:	00 00 
   25eaa:	48 8b 9c 24 80 00 00 	mov    0x80(%rsp),%rbx
   25eb1:	00 
   25eb2:	48 8d 04 1b          	lea    (%rbx,%rbx,1),%rax
   25eb6:	48 c1 e3 05          	shl    $0x5,%rbx
   25eba:	48 29 c3             	sub    %rax,%rbx
   25ebd:	48 c7 44 24 48 00 00 	movq   $0x0,0x48(%rsp)
   25ec4:	00 00 
   25ec6:	48 89 9c 24 98 00 00 	mov    %rbx,0x98(%rsp)
   25ecd:	00 
   25ece:	48 c7 84 24 b8 00 00 	movq   $0x0,0xb8(%rsp)
   25ed5:	00 00 00 00 00 
   25eda:	48 c7 84 24 c0 00 00 	movq   $0xa,0xc0(%rsp)
   25ee1:	00 0a 00 00 00 
   25ee6:	48 8d 44 24 10       	lea    0x10(%rsp),%rax
   25eeb:	48 89 84 24 a0 00 00 	mov    %rax,0xa0(%rsp)
   25ef2:	00 
   25ef3:	48 8d 84 24 98 00 00 	lea    0x98(%rsp),%rax
   25efa:	00 
   25efb:	48 89 84 24 a8 00 00 	mov    %rax,0xa8(%rsp)
   25f02:	00 
   25f03:	48 8d 44 24 48       	lea    0x48(%rsp),%rax
   25f08:	48 89 84 24 b0 00 00 	mov    %rax,0xb0(%rsp)
   25f0f:	00 
   25f10:	48 8d 7c 24 28       	lea    0x28(%rsp),%rdi
   25f15:	4c 8d bc 24 a0 00 00 	lea    0xa0(%rsp),%r15
   25f1c:	00 
   25f1d:	4c 89 fe             	mov    %r15,%rsi
   25f20:	e8 5b 10 00 00       	call   26f80 <<alloc::vec::Vec<f64> as alloc::vec::spec_from_iter_nested::SpecFromIterNested<f64, core::iter::adapters::map::Map<core::ops::range::Range<usize>, xdremux_engine::apple_photographic_styles::apple_style_revert_local_data::{closure#3}>>>::from_iter>
   25f25:	48 8b 6c 24 30       	mov    0x30(%rsp),%rbp
   25f2a:	4c 8b 44 24 38       	mov    0x38(%rsp),%r8
   25f2f:	ba 64 00 00 00       	mov    $0x64,%edx
   25f34:	4c 89 ff             	mov    %r15,%rdi
   25f37:	4c 89 f6             	mov    %r14,%rsi
   25f3a:	48 89 e9             	mov    %rbp,%rcx
   25f3d:	e8 9e dd ff ff       	call   23ce0 <xdremux_engine::apple_photographic_styles::solve_linear_system>
   25f42:	80 bc 24 a0 00 00 00 	cmpb   $0xff,0xa0(%rsp)
   25f49:	ff 
   25f4a:	74 34                	je     25f80 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x12a0>
   25f4c:	ff 15 d6 99 04 00    	call   *0x499d6(%rip)        # 6f928 <_DYNAMIC+0x2b8>
   25f52:	bf 50 00 00 00       	mov    $0x50,%edi
   25f57:	be 08 00 00 00       	mov    $0x8,%esi
   25f5c:	ff 15 e6 9a 04 00    	call   *0x49ae6(%rip)        # 6fa48 <_DYNAMIC+0x3d8>
   25f62:	48 85 c0             	test   %rax,%rax
   25f65:	0f 84 1f 0d 00 00    	je     26c8a <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1faa>
   25f6b:	49 89 c5             	mov    %rax,%r13
   25f6e:	48 83 c0 50          	add    $0x50,%rax
   25f72:	41 bf 0a 00 00 00    	mov    $0xa,%r15d
   25f78:	eb 2e                	jmp    25fa8 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x12c8>
   25f7a:	66 0f 1f 44 00 00    	nopw   0x0(%rax,%rax,1)
   25f80:	4c 8b bc 24 a8 00 00 	mov    0xa8(%rsp),%r15
   25f87:	00 
   25f88:	4c 8b ac 24 b0 00 00 	mov    0xb0(%rsp),%r13
   25f8f:	00 
   25f90:	48 8b 84 24 b8 00 00 	mov    0xb8(%rsp),%rax
   25f97:	00 
   25f98:	48 85 c0             	test   %rax,%rax
   25f9b:	74 7f                	je     2601c <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x133c>
   25f9d:	48 8d 04 c5 00 00 00 	lea    0x0(,%rax,8),%rax
   25fa4:	00 
   25fa5:	4c 01 e8             	add    %r13,%rax
   25fa8:	4c 8d 1d 61 6d fe ff 	lea    -0x1929f(%rip),%r11        # cd10 <GCC_except_table0+0x628>
   25faf:	66 0f 28 15 d9 6c fe 	movapd -0x19327(%rip),%xmm2        # cc90 <GCC_except_table0+0x5a8>
   25fb6:	ff 
   25fb7:	48 8b 8c 24 f8 00 00 	mov    0xf8(%rsp),%rcx
   25fbe:	00 
   25fbf:	48 8b b4 24 00 01 00 	mov    0x100(%rsp),%rsi
   25fc6:	00 
   25fc7:	31 d2                	xor    %edx,%edx
   25fc9:	4d 89 e8             	mov    %r13,%r8
   25fcc:	0f 1f 40 00          	nopl   0x0(%rax)
   25fd0:	4c 8b 4c 24 48       	mov    0x48(%rsp),%r9
   25fd5:	49 8d 3c 14          	lea    (%r12,%rdx,1),%rdi
   25fd9:	4c 01 cf             	add    %r9,%rdi
   25fdc:	48 39 f7             	cmp    %rsi,%rdi
   25fdf:	0f 83 32 0b 00 00    	jae    26b17 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1e37>
   25fe5:	49 01 d1             	add    %rdx,%r9
   25fe8:	45 31 d2             	xor    %r10d,%r10d
   25feb:	49 83 f9 0c          	cmp    $0xc,%r9
   25fef:	41 0f 93 c2          	setae  %r10b
   25ff3:	f2 43 0f 10 04 d3    	movsd  (%r11,%r10,8),%xmm0
   25ff9:	66 0f 28 c8          	movapd %xmm0,%xmm1
   25ffd:	66 0f 57 ca          	xorpd  %xmm2,%xmm1
   26001:	f2 41 0f 5f 08       	maxsd  (%r8),%xmm1
   26006:	49 83 c0 08          	add    $0x8,%r8
   2600a:	f2 0f 5d c1          	minsd  %xmm1,%xmm0
   2600e:	f2 0f 11 04 f9       	movsd  %xmm0,(%rcx,%rdi,8)
   26013:	48 83 c2 03          	add    $0x3,%rdx
   26017:	49 39 c0             	cmp    %rax,%r8
   2601a:	75 b4                	jne    25fd0 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x12f0>
   2601c:	4d 85 ff             	test   %r15,%r15
   2601f:	74 15                	je     26036 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1356>
   26021:	49 c1 e7 03          	shl    $0x3,%r15
   26025:	ba 08 00 00 00       	mov    $0x8,%edx
   2602a:	4c 89 ef             	mov    %r13,%rdi
   2602d:	4c 89 fe             	mov    %r15,%rsi
   26030:	ff 15 8a 98 04 00    	call   *0x4988a(%rip)        # 6f8c0 <_DYNAMIC+0x250>
   26036:	48 8b 74 24 28       	mov    0x28(%rsp),%rsi
   2603b:	48 85 f6             	test   %rsi,%rsi
   2603e:	74 12                	je     26052 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1372>
   26040:	48 c1 e6 03          	shl    $0x3,%rsi
   26044:	ba 08 00 00 00       	mov    $0x8,%edx
   26049:	48 89 ef             	mov    %rbp,%rdi
   2604c:	ff 15 6e 98 04 00    	call   *0x4986e(%rip)        # 6f8c0 <_DYNAMIC+0x250>
   26052:	48 c7 44 24 48 01 00 	movq   $0x1,0x48(%rsp)
   26059:	00 00 
   2605b:	48 89 9c 24 98 00 00 	mov    %rbx,0x98(%rsp)
   26062:	00 
   26063:	48 c7 84 24 b8 00 00 	movq   $0x0,0xb8(%rsp)
   2606a:	00 00 00 00 00 
   2606f:	48 c7 84 24 c0 00 00 	movq   $0xa,0xc0(%rsp)
   26076:	00 0a 00 00 00 
   2607b:	48 8d 44 24 10       	lea    0x10(%rsp),%rax
   26080:	48 89 84 24 a0 00 00 	mov    %rax,0xa0(%rsp)
   26087:	00 
   26088:	48 8d 84 24 98 00 00 	lea    0x98(%rsp),%rax
   2608f:	00 
   26090:	48 89 84 24 a8 00 00 	mov    %rax,0xa8(%rsp)
   26097:	00 
   26098:	48 8d 44 24 48       	lea    0x48(%rsp),%rax
   2609d:	48 89 84 24 b0 00 00 	mov    %rax,0xb0(%rsp)
   260a4:	00 
   260a5:	48 8d 7c 24 28       	lea    0x28(%rsp),%rdi
   260aa:	4c 8d bc 24 a0 00 00 	lea    0xa0(%rsp),%r15
   260b1:	00 
   260b2:	4c 89 fe             	mov    %r15,%rsi
   260b5:	e8 c6 0e 00 00       	call   26f80 <<alloc::vec::Vec<f64> as alloc::vec::spec_from_iter_nested::SpecFromIterNested<f64, core::iter::adapters::map::Map<core::ops::range::Range<usize>, xdremux_engine::apple_photographic_styles::apple_style_revert_local_data::{closure#3}>>>::from_iter>
   260ba:	48 8b 6c 24 30       	mov    0x30(%rsp),%rbp
   260bf:	4c 8b 44 24 38       	mov    0x38(%rsp),%r8
   260c4:	ba 64 00 00 00       	mov    $0x64,%edx
   260c9:	4c 89 ff             	mov    %r15,%rdi
   260cc:	4c 89 f6             	mov    %r14,%rsi
   260cf:	48 89 e9             	mov    %rbp,%rcx
   260d2:	e8 09 dc ff ff       	call   23ce0 <xdremux_engine::apple_photographic_styles::solve_linear_system>
   260d7:	80 bc 24 a0 00 00 00 	cmpb   $0xff,0xa0(%rsp)
   260de:	ff 
   260df:	74 2f                	je     26110 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1430>
   260e1:	ff 15 41 98 04 00    	call   *0x49841(%rip)        # 6f928 <_DYNAMIC+0x2b8>
   260e7:	bf 50 00 00 00       	mov    $0x50,%edi
   260ec:	be 08 00 00 00       	mov    $0x8,%esi
   260f1:	ff 15 51 99 04 00    	call   *0x49951(%rip)        # 6fa48 <_DYNAMIC+0x3d8>
   260f7:	48 85 c0             	test   %rax,%rax
   260fa:	0f 84 8a 0b 00 00    	je     26c8a <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1faa>
   26100:	49 89 c5             	mov    %rax,%r13
   26103:	48 83 c0 50          	add    $0x50,%rax
   26107:	41 bf 0a 00 00 00    	mov    $0xa,%r15d
   2610d:	eb 29                	jmp    26138 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1458>
   2610f:	90                   	nop
   26110:	4c 8b bc 24 a8 00 00 	mov    0xa8(%rsp),%r15
   26117:	00 
   26118:	4c 8b ac 24 b0 00 00 	mov    0xb0(%rsp),%r13
   2611f:	00 
   26120:	48 8b 84 24 b8 00 00 	mov    0xb8(%rsp),%rax
   26127:	00 
   26128:	48 85 c0             	test   %rax,%rax
   2612b:	74 7f                	je     261ac <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x14cc>
   2612d:	48 8d 04 c5 00 00 00 	lea    0x0(,%rax,8),%rax
   26134:	00 
   26135:	4c 01 e8             	add    %r13,%rax
   26138:	4c 8d 1d d1 6b fe ff 	lea    -0x1942f(%rip),%r11        # cd10 <GCC_except_table0+0x628>
   2613f:	66 0f 28 15 49 6b fe 	movapd -0x194b7(%rip),%xmm2        # cc90 <GCC_except_table0+0x5a8>
   26146:	ff 
   26147:	48 8b 8c 24 f8 00 00 	mov    0xf8(%rsp),%rcx
   2614e:	00 
   2614f:	48 8b b4 24 00 01 00 	mov    0x100(%rsp),%rsi
   26156:	00 
   26157:	31 d2                	xor    %edx,%edx
   26159:	4d 89 e8             	mov    %r13,%r8
   2615c:	0f 1f 40 00          	nopl   0x0(%rax)
   26160:	4c 8b 4c 24 48       	mov    0x48(%rsp),%r9
   26165:	49 8d 3c 14          	lea    (%r12,%rdx,1),%rdi
   26169:	4c 01 cf             	add    %r9,%rdi
   2616c:	48 39 f7             	cmp    %rsi,%rdi
   2616f:	0f 83 a2 09 00 00    	jae    26b17 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1e37>
   26175:	49 01 d1             	add    %rdx,%r9
   26178:	45 31 d2             	xor    %r10d,%r10d
   2617b:	49 83 f9 0c          	cmp    $0xc,%r9
   2617f:	41 0f 93 c2          	setae  %r10b
   26183:	f2 43 0f 10 04 d3    	movsd  (%r11,%r10,8),%xmm0
   26189:	66 0f 28 c8          	movapd %xmm0,%xmm1
   2618d:	66 0f 57 ca          	xorpd  %xmm2,%xmm1
   26191:	f2 41 0f 5f 08       	maxsd  (%r8),%xmm1
   26196:	49 83 c0 08          	add    $0x8,%r8
   2619a:	f2 0f 5d c1          	minsd  %xmm1,%xmm0
   2619e:	f2 0f 11 04 f9       	movsd  %xmm0,(%rcx,%rdi,8)
   261a3:	48 83 c2 03          	add    $0x3,%rdx
   261a7:	49 39 c0             	cmp    %rax,%r8
   261aa:	75 b4                	jne    26160 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1480>
   261ac:	4d 85 ff             	test   %r15,%r15
   261af:	74 15                	je     261c6 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x14e6>
   261b1:	49 c1 e7 03          	shl    $0x3,%r15
   261b5:	ba 08 00 00 00       	mov    $0x8,%edx
   261ba:	4c 89 ef             	mov    %r13,%rdi
   261bd:	4c 89 fe             	mov    %r15,%rsi
   261c0:	ff 15 fa 96 04 00    	call   *0x496fa(%rip)        # 6f8c0 <_DYNAMIC+0x250>
   261c6:	48 8b 74 24 28       	mov    0x28(%rsp),%rsi
   261cb:	48 85 f6             	test   %rsi,%rsi
   261ce:	74 12                	je     261e2 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1502>
   261d0:	48 c1 e6 03          	shl    $0x3,%rsi
   261d4:	ba 08 00 00 00       	mov    $0x8,%edx
   261d9:	48 89 ef             	mov    %rbp,%rdi
   261dc:	ff 15 de 96 04 00    	call   *0x496de(%rip)        # 6f8c0 <_DYNAMIC+0x250>
   261e2:	48 c7 44 24 48 02 00 	movq   $0x2,0x48(%rsp)
   261e9:	00 00 
   261eb:	48 89 9c 24 98 00 00 	mov    %rbx,0x98(%rsp)
   261f2:	00 
   261f3:	48 c7 84 24 b8 00 00 	movq   $0x0,0xb8(%rsp)
   261fa:	00 00 00 00 00 
   261ff:	48 c7 84 24 c0 00 00 	movq   $0xa,0xc0(%rsp)
   26206:	00 0a 00 00 00 
   2620b:	48 8d 44 24 10       	lea    0x10(%rsp),%rax
   26210:	48 89 84 24 a0 00 00 	mov    %rax,0xa0(%rsp)
   26217:	00 
   26218:	48 8d 84 24 98 00 00 	lea    0x98(%rsp),%rax
   2621f:	00 
   26220:	48 89 84 24 a8 00 00 	mov    %rax,0xa8(%rsp)
   26227:	00 
   26228:	48 8d 44 24 48       	lea    0x48(%rsp),%rax
   2622d:	48 89 84 24 b0 00 00 	mov    %rax,0xb0(%rsp)
   26234:	00 
   26235:	48 8d 7c 24 28       	lea    0x28(%rsp),%rdi
   2623a:	4c 8d bc 24 a0 00 00 	lea    0xa0(%rsp),%r15
   26241:	00 
   26242:	4c 89 fe             	mov    %r15,%rsi
   26245:	e8 36 0d 00 00       	call   26f80 <<alloc::vec::Vec<f64> as alloc::vec::spec_from_iter_nested::SpecFromIterNested<f64, core::iter::adapters::map::Map<core::ops::range::Range<usize>, xdremux_engine::apple_photographic_styles::apple_style_revert_local_data::{closure#3}>>>::from_iter>
   2624a:	48 8b 6c 24 30       	mov    0x30(%rsp),%rbp
   2624f:	4c 8b 44 24 38       	mov    0x38(%rsp),%r8
   26254:	ba 64 00 00 00       	mov    $0x64,%edx
   26259:	4c 89 ff             	mov    %r15,%rdi
   2625c:	4c 89 f6             	mov    %r14,%rsi
   2625f:	48 89 e9             	mov    %rbp,%rcx
   26262:	e8 79 da ff ff       	call   23ce0 <xdremux_engine::apple_photographic_styles::solve_linear_system>
   26267:	80 bc 24 a0 00 00 00 	cmpb   $0xff,0xa0(%rsp)
   2626e:	ff 
   2626f:	74 4f                	je     262c0 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x15e0>
   26271:	ff 15 b1 96 04 00    	call   *0x496b1(%rip)        # 6f928 <_DYNAMIC+0x2b8>
   26277:	bf 50 00 00 00       	mov    $0x50,%edi
   2627c:	be 08 00 00 00       	mov    $0x8,%esi
   26281:	ff 15 c1 97 04 00    	call   *0x497c1(%rip)        # 6fa48 <_DYNAMIC+0x3d8>
   26287:	48 85 c0             	test   %rax,%rax
   2628a:	0f 84 fa 09 00 00    	je     26c8a <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1faa>
   26290:	49 89 c5             	mov    %rax,%r13
   26293:	48 83 c0 50          	add    $0x50,%rax
   26297:	41 bf 0a 00 00 00    	mov    $0xa,%r15d
   2629d:	48 8b 1d 1c 96 04 00 	mov    0x4961c(%rip),%rbx        # 6f8c0 <_DYNAMIC+0x250>
   262a4:	4c 8d 1d 65 6a fe ff 	lea    -0x1959b(%rip),%r11        # cd10 <GCC_except_table0+0x628>
   262ab:	66 0f 28 15 dd 69 fe 	movapd -0x19623(%rip),%xmm2        # cc90 <GCC_except_table0+0x5a8>
   262b2:	ff 
   262b3:	eb 4d                	jmp    26302 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1622>
   262b5:	66 66 2e 0f 1f 84 00 	data16 cs nopw 0x0(%rax,%rax,1)
   262bc:	00 00 00 00 
   262c0:	4c 8b bc 24 a8 00 00 	mov    0xa8(%rsp),%r15
   262c7:	00 
   262c8:	4c 8b ac 24 b0 00 00 	mov    0xb0(%rsp),%r13
   262cf:	00 
   262d0:	48 8b 84 24 b8 00 00 	mov    0xb8(%rsp),%rax
   262d7:	00 
   262d8:	48 85 c0             	test   %rax,%rax
   262db:	4c 8d 1d 2e 6a fe ff 	lea    -0x195d2(%rip),%r11        # cd10 <GCC_except_table0+0x628>
   262e2:	66 0f 28 15 a6 69 fe 	movapd -0x1965a(%rip),%xmm2        # cc90 <GCC_except_table0+0x5a8>
   262e9:	ff 
   262ea:	0f 84 b3 00 00 00    	je     263a3 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x16c3>
   262f0:	48 8d 04 c5 00 00 00 	lea    0x0(,%rax,8),%rax
   262f7:	00 
   262f8:	4c 01 e8             	add    %r13,%rax
   262fb:	48 8b 1d be 95 04 00 	mov    0x495be(%rip),%rbx        # 6f8c0 <_DYNAMIC+0x250>
   26302:	48 8b 8c 24 f8 00 00 	mov    0xf8(%rsp),%rcx
   26309:	00 
   2630a:	48 8b b4 24 00 01 00 	mov    0x100(%rsp),%rsi
   26311:	00 
   26312:	31 d2                	xor    %edx,%edx
   26314:	4d 89 e8             	mov    %r13,%r8
   26317:	66 0f 1f 84 00 00 00 	nopw   0x0(%rax,%rax,1)
   2631e:	00 00 
   26320:	4c 8b 4c 24 48       	mov    0x48(%rsp),%r9
   26325:	49 8d 3c 14          	lea    (%r12,%rdx,1),%rdi
   26329:	4c 01 cf             	add    %r9,%rdi
   2632c:	48 39 f7             	cmp    %rsi,%rdi
   2632f:	0f 83 e2 07 00 00    	jae    26b17 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1e37>
   26335:	49 01 d1             	add    %rdx,%r9
   26338:	45 31 d2             	xor    %r10d,%r10d
   2633b:	49 83 f9 0c          	cmp    $0xc,%r9
   2633f:	41 0f 93 c2          	setae  %r10b
   26343:	f2 43 0f 10 04 d3    	movsd  (%r11,%r10,8),%xmm0
   26349:	66 0f 28 c8          	movapd %xmm0,%xmm1
   2634d:	66 0f 57 ca          	xorpd  %xmm2,%xmm1
   26351:	f2 41 0f 5f 08       	maxsd  (%r8),%xmm1
   26356:	49 83 c0 08          	add    $0x8,%r8
   2635a:	f2 0f 5d c1          	minsd  %xmm1,%xmm0
   2635e:	f2 0f 11 04 f9       	movsd  %xmm0,(%rcx,%rdi,8)
   26363:	48 83 c2 03          	add    $0x3,%rdx
   26367:	49 39 c0             	cmp    %rax,%r8
   2636a:	75 b4                	jne    26320 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1640>
   2636c:	4d 85 ff             	test   %r15,%r15
   2636f:	74 11                	je     26382 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x16a2>
   26371:	49 c1 e7 03          	shl    $0x3,%r15
   26375:	ba 08 00 00 00       	mov    $0x8,%edx
   2637a:	4c 89 ef             	mov    %r13,%rdi
   2637d:	4c 89 fe             	mov    %r15,%rsi
   26380:	ff d3                	call   *%rbx
   26382:	48 8b 74 24 28       	mov    0x28(%rsp),%rsi
   26387:	48 85 f6             	test   %rsi,%rsi
   2638a:	0f 84 f0 f7 ff ff    	je     25b80 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0xea0>
   26390:	48 c1 e6 03          	shl    $0x3,%rsi
   26394:	ba 08 00 00 00       	mov    $0x8,%edx
   26399:	48 89 ef             	mov    %rbp,%rdi
   2639c:	ff d3                	call   *%rbx
   2639e:	e9 dd f7 ff ff       	jmp    25b80 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0xea0>
   263a3:	48 8b 1d 16 95 04 00 	mov    0x49516(%rip),%rbx        # 6f8c0 <_DYNAMIC+0x250>
   263aa:	4d 85 ff             	test   %r15,%r15
   263ad:	75 c2                	jne    26371 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1691>
   263af:	eb d1                	jmp    26382 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x16a2>
   263b1:	48 8b 84 24 d0 01 00 	mov    0x1d0(%rsp),%rax
   263b8:	00 
   263b9:	48 05 e0 e3 ff ff    	add    $0xffffffffffffe3e0,%rax
   263bf:	48 8b b4 24 48 01 00 	mov    0x148(%rsp),%rsi
   263c6:	00 
   263c7:	48 81 c6 00 e1 00 00 	add    $0xe100,%rsi
   263ce:	48 8b 94 24 50 01 00 	mov    0x150(%rsp),%rdx
   263d5:	00 
   263d6:	48 81 c2 20 1c 00 00 	add    $0x1c20,%rdx
   263dd:	48 8b 8c 24 58 01 00 	mov    0x158(%rsp),%rcx
   263e4:	00 
   263e5:	48 81 c1 70 08 00 00 	add    $0x870,%rcx
   263ec:	48 8b bc 24 28 01 00 	mov    0x128(%rsp),%rdi
   263f3:	00 
   263f4:	48 83 ff 0c          	cmp    $0xc,%rdi
   263f8:	0f 85 a3 f6 ff ff    	jne    25aa1 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0xdc1>
   263fe:	ff 15 24 95 04 00    	call   *0x49524(%rip)        # 6f928 <_DYNAMIC+0x2b8>
   26404:	41 bf 80 ca 00 00    	mov    $0xca80,%r15d
   2640a:	41 bd 01 00 00 00    	mov    $0x1,%r13d
   26410:	bf 80 ca 00 00       	mov    $0xca80,%edi
   26415:	be 01 00 00 00       	mov    $0x1,%esi
   2641a:	ff 15 10 95 04 00    	call   *0x49510(%rip)        # 6f930 <_DYNAMIC+0x2c0>
   26420:	48 85 c0             	test   %rax,%rax
   26423:	0f 84 73 08 00 00    	je     26c9c <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1fbc>
   26429:	48 89 c5             	mov    %rax,%rbp
   2642c:	48 c7 44 24 28 80 ca 	movq   $0xca80,0x28(%rsp)
   26433:	00 00 
   26435:	48 89 44 24 30       	mov    %rax,0x30(%rsp)
   2643a:	48 c7 44 24 38 00 00 	movq   $0x0,0x38(%rsp)
   26441:	00 00 
   26443:	48 8b 84 24 f8 00 00 	mov    0xf8(%rsp),%rax
   2644a:	00 
   2644b:	48 8b 8c 24 00 01 00 	mov    0x100(%rsp),%rcx
   26452:	00 
   26453:	48 89 4c 24 50       	mov    %rcx,0x50(%rsp)
   26458:	48 89 84 24 60 01 00 	mov    %rax,0x160(%rsp)
   2645f:	00 
   26460:	4c 8d 60 10          	lea    0x10(%rax),%r12
   26464:	31 db                	xor    %ebx,%ebx
   26466:	45 31 ed             	xor    %r13d,%r13d
   26469:	31 d2                	xor    %edx,%edx
   2646b:	e9 87 00 00 00       	jmp    264f7 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1817>
   26470:	48 01 ef             	add    %rbp,%rdi
   26473:	48 8d 15 ae 6e 04 00 	lea    0x46eae(%rip),%rdx        # 6d328 <anon.441e142b17a370d935234e81e4cac821.6.llvm.308828945136828694+0x198>
   2647a:	be 80 51 01 00       	mov    $0x15180,%esi
   2647f:	eb 12                	jmp    26493 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x17b3>
   26481:	49 01 ef             	add    %rbp,%r15
   26484:	48 8d 15 b5 6e 04 00 	lea    0x46eb5(%rip),%rdx        # 6d340 <anon.441e142b17a370d935234e81e4cac821.6.llvm.308828945136828694+0x1b0>
   2648b:	be 64 00 00 00       	mov    $0x64,%esi
   26490:	4c 89 ff             	mov    %r15,%rdi
   26493:	4c 8b 74 24 08       	mov    0x8(%rsp),%r14
   26498:	ff 15 ca 95 04 00    	call   *0x495ca(%rip)        # 6fa68 <_DYNAMIC+0x3f8>
   2649e:	e9 27 08 00 00       	jmp    26cca <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1fea>
   264a3:	4c 89 c7             	mov    %r8,%rdi
   264a6:	eb 0d                	jmp    264b5 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x17d5>
   264a8:	4c 89 ef             	mov    %r13,%rdi
   264ab:	eb 08                	jmp    264b5 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x17d5>
   264ad:	48 89 ef             	mov    %rbp,%rdi
   264b0:	eb 03                	jmp    264b5 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x17d5>
   264b2:	4c 89 e7             	mov    %r12,%rdi
   264b5:	48 8d 15 e4 6e 04 00 	lea    0x46ee4(%rip),%rdx        # 6d3a0 <anon.441e142b17a370d935234e81e4cac821.6.llvm.308828945136828694+0x210>
   264bc:	ff 15 a6 95 04 00    	call   *0x495a6(%rip)        # 6fa68 <_DYNAMIC+0x3f8>
   264c2:	e9 03 08 00 00       	jmp    26cca <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1fea>
   264c7:	4c 8b a4 24 80 01 00 	mov    0x180(%rsp),%r12
   264ce:	00 
   264cf:	49 81 c4 80 43 00 00 	add    $0x4380,%r12
   264d6:	48 8b 9c 24 70 01 00 	mov    0x170(%rsp),%rbx
   264dd:	00 
   264de:	48 81 c3 70 08 00 00 	add    $0x870,%rbx
   264e5:	48 8b 94 24 30 01 00 	mov    0x130(%rsp),%rdx
   264ec:	00 
   264ed:	48 83 fa 0c          	cmp    $0xc,%rdx
   264f1:	0f 84 49 06 00 00    	je     26b40 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1e60>
   264f7:	48 ff c2             	inc    %rdx
   264fa:	48 89 94 24 30 01 00 	mov    %rdx,0x130(%rsp)
   26501:	00 
   26502:	48 89 9c 24 70 01 00 	mov    %rbx,0x170(%rsp)
   26509:	00 
   2650a:	4c 89 a4 24 80 01 00 	mov    %r12,0x180(%rsp)
   26511:	00 
   26512:	31 c0                	xor    %eax,%eax
   26514:	eb 29                	jmp    2653f <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x185f>
   26516:	4c 8b a4 24 10 01 00 	mov    0x110(%rsp),%r12
   2651d:	00 
   2651e:	49 81 c4 80 07 00 00 	add    $0x780,%r12
   26525:	48 8b 9c 24 80 00 00 	mov    0x80(%rsp),%rbx
   2652c:	00 
   2652d:	48 81 c3 f0 00 00 00 	add    $0xf0,%rbx
   26534:	48 8b 44 24 70       	mov    0x70(%rsp),%rax
   26539:	48 83 f8 09          	cmp    $0x9,%rax
   2653d:	74 88                	je     264c7 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x17e7>
   2653f:	48 ff c0             	inc    %rax
   26542:	48 89 44 24 70       	mov    %rax,0x70(%rsp)
   26547:	48 89 9c 24 80 00 00 	mov    %rbx,0x80(%rsp)
   2654e:	00 
   2654f:	4c 89 a4 24 10 01 00 	mov    %r12,0x110(%rsp)
   26556:	00 
   26557:	31 c0                	xor    %eax,%eax
   26559:	eb 1b                	jmp    26576 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1896>
   2655b:	0f 1f 44 00 00       	nopl   0x0(%rax,%rax,1)
   26560:	49 81 c4 f0 00 00 00 	add    $0xf0,%r12
   26567:	48 83 c3 1e          	add    $0x1e,%rbx
   2656b:	48 8b 44 24 60       	mov    0x60(%rsp),%rax
   26570:	48 83 f8 08          	cmp    $0x8,%rax
   26574:	74 a0                	je     26516 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1836>
   26576:	48 ff c0             	inc    %rax
   26579:	48 89 44 24 60       	mov    %rax,0x60(%rsp)
   2657e:	49 c7 c7 fd ff ff ff 	mov    $0xfffffffffffffffd,%r15
   26585:	66 66 2e 0f 1f 84 00 	data16 cs nopw 0x0(%rax,%rax,1)
   2658c:	00 00 00 00 
   26590:	66 0f 57 c0          	xorpd  %xmm0,%xmm0
   26594:	f3 0f 11 44 24 08    	movss  %xmm0,0x8(%rsp)
   2659a:	49 83 ff 08          	cmp    $0x8,%r15
   2659e:	77 13                	ja     265b3 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x18d3>
   265a0:	48 8d 05 6d 92 fe ff 	lea    -0x16d93(%rip),%rax        # f814 <anon.441e142b17a370d935234e81e4cac821.4.llvm.308828945136828694+0x840>
   265a7:	f3 42 0f 10 04 b8    	movss  (%rax,%r15,4),%xmm0
   265ad:	f3 0f 11 44 24 08    	movss  %xmm0,0x8(%rsp)
   265b3:	4a 8d 3c 3b          	lea    (%rbx,%r15,1),%rdi
   265b7:	48 83 c7 03          	add    $0x3,%rdi
   265bb:	48 3b 7c 24 50       	cmp    0x50(%rsp),%rdi
   265c0:	0f 83 63 05 00 00    	jae    26b29 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1e49>
   265c6:	f2 43 0f 10 44 fc 08 	movsd  0x8(%r12,%r15,8),%xmm0
   265cd:	f2 0f 5a c0          	cvtsd2ss %xmm0,%xmm0
   265d1:	f3 0f 10 4c 24 08    	movss  0x8(%rsp),%xmm1
   265d7:	f3 0f 58 c8          	addss  %xmm0,%xmm1
   265db:	f3 0f 11 4c 24 08    	movss  %xmm1,0x8(%rsp)
   265e1:	48 8b 05 48 96 04 00 	mov    0x49648(%rip),%rax        # 6fc30 <_DYNAMIC+0x5c0>
   265e8:	48 8b 00             	mov    (%rax),%rax
   265eb:	48 85 c0             	test   %rax,%rax
   265ee:	0f 84 f8 00 00 00    	je     266ec <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1a0c>
   265f4:	48 b9 00 00 00 00 00 	movabs $0x40000000000000,%rcx
   265fb:	00 40 00 
   265fe:	48 85 c8             	test   %rcx,%rax
   26601:	0f 85 f9 00 00 00    	jne    26700 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1a20>
   26607:	66 0f 6e 44 24 08    	movd   0x8(%rsp),%xmm0
   2660d:	66 0f 7e c1          	movd   %xmm0,%ecx
   26611:	89 c8                	mov    %ecx,%eax
   26613:	25 00 00 00 80       	and    $0x80000000,%eax
   26618:	89 ce                	mov    %ecx,%esi
   2661a:	81 e6 00 00 80 7f    	and    $0x7f800000,%esi
   26620:	89 ca                	mov    %ecx,%edx
   26622:	81 e2 ff ff 7f 00    	and    $0x7fffff,%edx
   26628:	81 fe 00 00 80 7f    	cmp    $0x7f800000,%esi
   2662e:	75 30                	jne    26660 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1980>
   26630:	31 c9                	xor    %ecx,%ecx
   26632:	85 d2                	test   %edx,%edx
   26634:	0f 95 c1             	setne  %cl
   26637:	c1 e1 09             	shl    $0x9,%ecx
   2663a:	c1 e8 10             	shr    $0x10,%eax
   2663d:	c1 ea 0d             	shr    $0xd,%edx
   26640:	09 c2                	or     %eax,%edx
   26642:	09 ca                	or     %ecx,%edx
   26644:	81 ca 00 7c 00 00    	or     $0x7c00,%edx
   2664a:	41 89 d6             	mov    %edx,%r14d
   2664d:	e9 bc 00 00 00       	jmp    2670e <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1a2e>
   26652:	66 66 66 66 66 2e 0f 	data16 data16 data16 data16 cs nopw 0x0(%rax,%rax,1)
   26659:	1f 84 00 00 00 00 00 
   26660:	c1 e8 10             	shr    $0x10,%eax
   26663:	81 fe 00 00 00 47    	cmp    $0x47000000,%esi
   26669:	76 0d                	jbe    26678 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1998>
   2666b:	0d 00 7c 00 00       	or     $0x7c00,%eax
   26670:	41 89 c6             	mov    %eax,%r14d
   26673:	e9 96 00 00 00       	jmp    2670e <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1a2e>
   26678:	81 fe 00 00 80 38    	cmp    $0x38800000,%esi
   2667e:	73 42                	jae    266c2 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x19e2>
   26680:	81 fe 00 00 00 33    	cmp    $0x33000000,%esi
   26686:	72 e8                	jb     26670 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1990>
   26688:	c1 ee 17             	shr    $0x17,%esi
   2668b:	81 ca 00 00 80 00    	or     $0x800000,%edx
   26691:	b1 7e                	mov    $0x7e,%cl
   26693:	40 28 f1             	sub    %sil,%cl
   26696:	41 89 d6             	mov    %edx,%r14d
   26699:	41 d3 ee             	shr    %cl,%r14d
   2669c:	b9 1d 00 00 00       	mov    $0x1d,%ecx
   266a1:	29 f1                	sub    %esi,%ecx
   266a3:	83 e1 1f             	and    $0x1f,%ecx
   266a6:	0f a3 ca             	bt     %ecx,%edx
   266a9:	73 12                	jae    266bd <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x19dd>
   266ab:	be 03 00 00 00       	mov    $0x3,%esi
   266b0:	d3 e6                	shl    %cl,%esi
   266b2:	ff ce                	dec    %esi
   266b4:	21 d6                	and    %edx,%esi
   266b6:	83 fe 01             	cmp    $0x1,%esi
   266b9:	41 83 de ff          	sbb    $0xffffffff,%r14d
   266bd:	41 09 c6             	or     %eax,%r14d
   266c0:	eb 4c                	jmp    2670e <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1a2e>
   266c2:	c1 ee 0d             	shr    $0xd,%esi
   266c5:	c1 ea 0d             	shr    $0xd,%edx
   266c8:	89 cf                	mov    %ecx,%edi
   266ca:	c1 ef 0c             	shr    $0xc,%edi
   266cd:	f7 c1 ff 2f 00 00    	test   $0x2fff,%ecx
   266d3:	0f 95 c1             	setne  %cl
   266d6:	40 20 f9             	and    %dil,%cl
   266d9:	01 f2                	add    %esi,%edx
   266db:	81 c2 00 40 00 00    	add    $0x4000,%edx
   266e1:	09 c2                	or     %eax,%edx
   266e3:	44 0f b6 f1          	movzbl %cl,%r14d
   266e7:	41 01 d6             	add    %edx,%r14d
   266ea:	eb 22                	jmp    2670e <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1a2e>
   266ec:	ff 15 46 95 04 00    	call   *0x49546(%rip)        # 6fc38 <_DYNAMIC+0x5c8>
   266f2:	48 0f ba e0 36       	bt     $0x36,%rax
   266f7:	0f 83 0a ff ff ff    	jae    26607 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1927>
   266fd:	0f 1f 00             	nopl   (%rax)
   26700:	66 0f 6e 44 24 08    	movd   0x8(%rsp),%xmm0
   26706:	e8 d5 06 00 00       	call   26de0 <half::binary16::arch::x86::f32_to_f16_x86_f16c>
   2670b:	41 89 c6             	mov    %eax,%r14d
   2670e:	48 8b 44 24 28       	mov    0x28(%rsp),%rax
   26713:	4c 29 e8             	sub    %r13,%rax
   26716:	48 83 f8 01          	cmp    $0x1,%rax
   2671a:	0f 86 77 01 00 00    	jbe    26897 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1bb7>
   26720:	66 46 89 74 2d 00    	mov    %r14w,0x0(%rbp,%r13,1)
   26726:	49 83 c5 02          	add    $0x2,%r13
   2672a:	4c 89 6c 24 38       	mov    %r13,0x38(%rsp)
   2672f:	49 8d 47 01          	lea    0x1(%r15),%rax
   26733:	66 0f ef c0          	pxor   %xmm0,%xmm0
   26737:	66 0f 7e 44 24 08    	movd   %xmm0,0x8(%rsp)
   2673d:	48 83 f8 08          	cmp    $0x8,%rax
   26741:	77 14                	ja     26757 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1a77>
   26743:	48 8d 05 ca 90 fe ff 	lea    -0x16f36(%rip),%rax        # f814 <anon.441e142b17a370d935234e81e4cac821.4.llvm.308828945136828694+0x840>
   2674a:	66 42 0f 6e 44 b8 04 	movd   0x4(%rax,%r15,4),%xmm0
   26751:	66 0f 7e 44 24 08    	movd   %xmm0,0x8(%rsp)
   26757:	4a 8d 3c 3b          	lea    (%rbx,%r15,1),%rdi
   2675b:	48 83 c7 04          	add    $0x4,%rdi
   2675f:	48 3b 7c 24 50       	cmp    0x50(%rsp),%rdi
   26764:	0f 83 bf 03 00 00    	jae    26b29 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1e49>
   2676a:	f2 43 0f 10 44 fc 10 	movsd  0x10(%r12,%r15,8),%xmm0
   26771:	f2 0f 5a c0          	cvtsd2ss %xmm0,%xmm0
   26775:	f3 0f 10 4c 24 08    	movss  0x8(%rsp),%xmm1
   2677b:	f3 0f 58 c8          	addss  %xmm0,%xmm1
   2677f:	f3 0f 11 4c 24 08    	movss  %xmm1,0x8(%rsp)
   26785:	48 8b 05 a4 94 04 00 	mov    0x494a4(%rip),%rax        # 6fc30 <_DYNAMIC+0x5c0>
   2678c:	48 8b 00             	mov    (%rax),%rax
   2678f:	48 85 c0             	test   %rax,%rax
   26792:	0f 84 2b 01 00 00    	je     268c3 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1be3>
   26798:	48 b9 00 00 00 00 00 	movabs $0x40000000000000,%rcx
   2679f:	00 40 00 
   267a2:	48 85 c8             	test   %rcx,%rax
   267a5:	0f 85 35 01 00 00    	jne    268e0 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1c00>
   267ab:	66 0f 6e 44 24 08    	movd   0x8(%rsp),%xmm0
   267b1:	66 0f 7e c1          	movd   %xmm0,%ecx
   267b5:	89 c8                	mov    %ecx,%eax
   267b7:	25 00 00 00 80       	and    $0x80000000,%eax
   267bc:	89 ce                	mov    %ecx,%esi
   267be:	81 e6 00 00 80 7f    	and    $0x7f800000,%esi
   267c4:	89 ca                	mov    %ecx,%edx
   267c6:	81 e2 ff ff 7f 00    	and    $0x7fffff,%edx
   267cc:	81 fe 00 00 80 7f    	cmp    $0x7f800000,%esi
   267d2:	75 2c                	jne    26800 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1b20>
   267d4:	31 c9                	xor    %ecx,%ecx
   267d6:	85 d2                	test   %edx,%edx
   267d8:	0f 95 c1             	setne  %cl
   267db:	c1 e1 09             	shl    $0x9,%ecx
   267de:	c1 e8 10             	shr    $0x10,%eax
   267e1:	c1 ea 0d             	shr    $0xd,%edx
   267e4:	09 c2                	or     %eax,%edx
   267e6:	09 ca                	or     %ecx,%edx
   267e8:	81 ca 00 7c 00 00    	or     $0x7c00,%edx
   267ee:	41 89 d6             	mov    %edx,%r14d
   267f1:	e9 f8 00 00 00       	jmp    268ee <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1c0e>
   267f6:	66 2e 0f 1f 84 00 00 	cs nopw 0x0(%rax,%rax,1)
   267fd:	00 00 00 
   26800:	c1 e8 10             	shr    $0x10,%eax
   26803:	81 fe 00 00 00 47    	cmp    $0x47000000,%esi
   26809:	76 0d                	jbe    26818 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1b38>
   2680b:	0d 00 7c 00 00       	or     $0x7c00,%eax
   26810:	41 89 c6             	mov    %eax,%r14d
   26813:	e9 d6 00 00 00       	jmp    268ee <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1c0e>
   26818:	81 fe 00 00 80 38    	cmp    $0x38800000,%esi
   2681e:	73 10                	jae    26830 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1b50>
   26820:	81 fe 00 00 00 33    	cmp    $0x33000000,%esi
   26826:	73 35                	jae    2685d <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1b7d>
   26828:	41 89 c6             	mov    %eax,%r14d
   2682b:	e9 be 00 00 00       	jmp    268ee <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1c0e>
   26830:	c1 ee 0d             	shr    $0xd,%esi
   26833:	c1 ea 0d             	shr    $0xd,%edx
   26836:	89 cf                	mov    %ecx,%edi
   26838:	c1 ef 0c             	shr    $0xc,%edi
   2683b:	f7 c1 ff 2f 00 00    	test   $0x2fff,%ecx
   26841:	0f 95 c1             	setne  %cl
   26844:	40 20 f9             	and    %dil,%cl
   26847:	01 f2                	add    %esi,%edx
   26849:	81 c2 00 40 00 00    	add    $0x4000,%edx
   2684f:	09 c2                	or     %eax,%edx
   26851:	44 0f b6 f1          	movzbl %cl,%r14d
   26855:	41 01 d6             	add    %edx,%r14d
   26858:	e9 91 00 00 00       	jmp    268ee <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1c0e>
   2685d:	c1 ee 17             	shr    $0x17,%esi
   26860:	81 ca 00 00 80 00    	or     $0x800000,%edx
   26866:	b1 7e                	mov    $0x7e,%cl
   26868:	40 28 f1             	sub    %sil,%cl
   2686b:	41 89 d6             	mov    %edx,%r14d
   2686e:	41 d3 ee             	shr    %cl,%r14d
   26871:	b9 1d 00 00 00       	mov    $0x1d,%ecx
   26876:	29 f1                	sub    %esi,%ecx
   26878:	83 e1 1f             	and    $0x1f,%ecx
   2687b:	0f a3 ca             	bt     %ecx,%edx
   2687e:	73 12                	jae    26892 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1bb2>
   26880:	be 03 00 00 00       	mov    $0x3,%esi
   26885:	d3 e6                	shl    %cl,%esi
   26887:	ff ce                	dec    %esi
   26889:	21 d6                	and    %edx,%esi
   2688b:	83 fe 01             	cmp    $0x1,%esi
   2688e:	41 83 de ff          	sbb    $0xffffffff,%r14d
   26892:	41 09 c6             	or     %eax,%r14d
   26895:	eb 57                	jmp    268ee <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1c0e>
   26897:	ba 02 00 00 00       	mov    $0x2,%edx
   2689c:	b9 01 00 00 00       	mov    $0x1,%ecx
   268a1:	41 b8 01 00 00 00    	mov    $0x1,%r8d
   268a7:	48 8d 7c 24 28       	lea    0x28(%rsp),%rdi
   268ac:	4c 89 ee             	mov    %r13,%rsi
   268af:	e8 1c 08 00 00       	call   270d0 <<alloc::raw_vec::RawVecInner<_>>::reserve::do_reserve_and_handle::<alloc::alloc::Global>>
   268b4:	48 8b 6c 24 30       	mov    0x30(%rsp),%rbp
   268b9:	4c 8b 6c 24 38       	mov    0x38(%rsp),%r13
   268be:	e9 5d fe ff ff       	jmp    26720 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1a40>
   268c3:	ff 15 6f 93 04 00    	call   *0x4936f(%rip)        # 6fc38 <_DYNAMIC+0x5c8>
   268c9:	48 0f ba e0 36       	bt     $0x36,%rax
   268ce:	0f 83 d7 fe ff ff    	jae    267ab <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1acb>
   268d4:	66 66 66 2e 0f 1f 84 	data16 data16 cs nopw 0x0(%rax,%rax,1)
   268db:	00 00 00 00 00 
   268e0:	66 0f 6e 44 24 08    	movd   0x8(%rsp),%xmm0
   268e6:	e8 f5 04 00 00       	call   26de0 <half::binary16::arch::x86::f32_to_f16_x86_f16c>
   268eb:	41 89 c6             	mov    %eax,%r14d
   268ee:	48 8b 44 24 28       	mov    0x28(%rsp),%rax
   268f3:	4c 29 e8             	sub    %r13,%rax
   268f6:	48 83 f8 01          	cmp    $0x1,%rax
   268fa:	0f 86 77 01 00 00    	jbe    26a77 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1d97>
   26900:	48 8b 6c 24 30       	mov    0x30(%rsp),%rbp
   26905:	66 46 89 74 2d 00    	mov    %r14w,0x0(%rbp,%r13,1)
   2690b:	49 83 c5 02          	add    $0x2,%r13
   2690f:	4c 89 6c 24 38       	mov    %r13,0x38(%rsp)
   26914:	49 8d 47 02          	lea    0x2(%r15),%rax
   26918:	66 0f ef c0          	pxor   %xmm0,%xmm0
   2691c:	66 0f 7e 44 24 08    	movd   %xmm0,0x8(%rsp)
   26922:	48 83 f8 08          	cmp    $0x8,%rax
   26926:	77 14                	ja     2693c <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1c5c>
   26928:	48 8d 05 e5 8e fe ff 	lea    -0x1711b(%rip),%rax        # f814 <anon.441e142b17a370d935234e81e4cac821.4.llvm.308828945136828694+0x840>
   2692f:	66 42 0f 6e 44 b8 08 	movd   0x8(%rax,%r15,4),%xmm0
   26936:	66 0f 7e 44 24 08    	movd   %xmm0,0x8(%rsp)
   2693c:	4a 8d 3c 3b          	lea    (%rbx,%r15,1),%rdi
   26940:	48 83 c7 05          	add    $0x5,%rdi
   26944:	48 3b 7c 24 50       	cmp    0x50(%rsp),%rdi
   26949:	0f 83 da 01 00 00    	jae    26b29 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1e49>
   2694f:	f2 43 0f 10 44 fc 18 	movsd  0x18(%r12,%r15,8),%xmm0
   26956:	f2 0f 5a c0          	cvtsd2ss %xmm0,%xmm0
   2695a:	f3 0f 10 4c 24 08    	movss  0x8(%rsp),%xmm1
   26960:	f3 0f 58 c8          	addss  %xmm0,%xmm1
   26964:	f3 0f 11 4c 24 08    	movss  %xmm1,0x8(%rsp)
   2696a:	48 8b 05 bf 92 04 00 	mov    0x492bf(%rip),%rax        # 6fc30 <_DYNAMIC+0x5c0>
   26971:	48 8b 00             	mov    (%rax),%rax
   26974:	48 85 c0             	test   %rax,%rax
   26977:	0f 84 21 01 00 00    	je     26a9e <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1dbe>
   2697d:	48 b9 00 00 00 00 00 	movabs $0x40000000000000,%rcx
   26984:	00 40 00 
   26987:	48 85 c8             	test   %rcx,%rax
   2698a:	0f 85 20 01 00 00    	jne    26ab0 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1dd0>
   26990:	66 0f 6e 44 24 08    	movd   0x8(%rsp),%xmm0
   26996:	66 0f 7e c1          	movd   %xmm0,%ecx
   2699a:	89 c8                	mov    %ecx,%eax
   2699c:	25 00 00 00 80       	and    $0x80000000,%eax
   269a1:	89 ce                	mov    %ecx,%esi
   269a3:	81 e6 00 00 80 7f    	and    $0x7f800000,%esi
   269a9:	89 ca                	mov    %ecx,%edx
   269ab:	81 e2 ff ff 7f 00    	and    $0x7fffff,%edx
   269b1:	81 fe 00 00 80 7f    	cmp    $0x7f800000,%esi
   269b7:	75 27                	jne    269e0 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1d00>
   269b9:	31 c9                	xor    %ecx,%ecx
   269bb:	85 d2                	test   %edx,%edx
   269bd:	0f 95 c1             	setne  %cl
   269c0:	c1 e1 09             	shl    $0x9,%ecx
   269c3:	c1 e8 10             	shr    $0x10,%eax
   269c6:	c1 ea 0d             	shr    $0xd,%edx
   269c9:	09 c2                	or     %eax,%edx
   269cb:	09 ca                	or     %ecx,%edx
   269cd:	81 ca 00 7c 00 00    	or     $0x7c00,%edx
   269d3:	41 89 d6             	mov    %edx,%r14d
   269d6:	e9 e3 00 00 00       	jmp    26abe <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1dde>
   269db:	0f 1f 44 00 00       	nopl   0x0(%rax,%rax,1)
   269e0:	c1 e8 10             	shr    $0x10,%eax
   269e3:	81 fe 00 00 00 47    	cmp    $0x47000000,%esi
   269e9:	76 0d                	jbe    269f8 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1d18>
   269eb:	0d 00 7c 00 00       	or     $0x7c00,%eax
   269f0:	41 89 c6             	mov    %eax,%r14d
   269f3:	e9 c6 00 00 00       	jmp    26abe <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1dde>
   269f8:	81 fe 00 00 80 38    	cmp    $0x38800000,%esi
   269fe:	73 10                	jae    26a10 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1d30>
   26a00:	81 fe 00 00 00 33    	cmp    $0x33000000,%esi
   26a06:	73 35                	jae    26a3d <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1d5d>
   26a08:	41 89 c6             	mov    %eax,%r14d
   26a0b:	e9 ae 00 00 00       	jmp    26abe <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1dde>
   26a10:	c1 ee 0d             	shr    $0xd,%esi
   26a13:	c1 ea 0d             	shr    $0xd,%edx
   26a16:	89 cf                	mov    %ecx,%edi
   26a18:	c1 ef 0c             	shr    $0xc,%edi
   26a1b:	f7 c1 ff 2f 00 00    	test   $0x2fff,%ecx
   26a21:	0f 95 c1             	setne  %cl
   26a24:	40 20 f9             	and    %dil,%cl
   26a27:	01 f2                	add    %esi,%edx
   26a29:	81 c2 00 40 00 00    	add    $0x4000,%edx
   26a2f:	09 c2                	or     %eax,%edx
   26a31:	44 0f b6 f1          	movzbl %cl,%r14d
   26a35:	41 01 d6             	add    %edx,%r14d
   26a38:	e9 81 00 00 00       	jmp    26abe <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1dde>
   26a3d:	c1 ee 17             	shr    $0x17,%esi
   26a40:	81 ca 00 00 80 00    	or     $0x800000,%edx
   26a46:	b1 7e                	mov    $0x7e,%cl
   26a48:	40 28 f1             	sub    %sil,%cl
   26a4b:	41 89 d6             	mov    %edx,%r14d
   26a4e:	41 d3 ee             	shr    %cl,%r14d
   26a51:	b9 1d 00 00 00       	mov    $0x1d,%ecx
   26a56:	29 f1                	sub    %esi,%ecx
   26a58:	83 e1 1f             	and    $0x1f,%ecx
   26a5b:	0f a3 ca             	bt     %ecx,%edx
   26a5e:	73 12                	jae    26a72 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1d92>
   26a60:	be 03 00 00 00       	mov    $0x3,%esi
   26a65:	d3 e6                	shl    %cl,%esi
   26a67:	ff ce                	dec    %esi
   26a69:	21 d6                	and    %edx,%esi
   26a6b:	83 fe 01             	cmp    $0x1,%esi
   26a6e:	41 83 de ff          	sbb    $0xffffffff,%r14d
   26a72:	41 09 c6             	or     %eax,%r14d
   26a75:	eb 47                	jmp    26abe <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1dde>
   26a77:	ba 02 00 00 00       	mov    $0x2,%edx
   26a7c:	b9 01 00 00 00       	mov    $0x1,%ecx
   26a81:	41 b8 01 00 00 00    	mov    $0x1,%r8d
   26a87:	48 8d 7c 24 28       	lea    0x28(%rsp),%rdi
   26a8c:	4c 89 ee             	mov    %r13,%rsi
   26a8f:	e8 3c 06 00 00       	call   270d0 <<alloc::raw_vec::RawVecInner<_>>::reserve::do_reserve_and_handle::<alloc::alloc::Global>>
   26a94:	4c 8b 6c 24 38       	mov    0x38(%rsp),%r13
   26a99:	e9 62 fe ff ff       	jmp    26900 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1c20>
   26a9e:	ff 15 94 91 04 00    	call   *0x49194(%rip)        # 6fc38 <_DYNAMIC+0x5c8>
   26aa4:	48 0f ba e0 36       	bt     $0x36,%rax
   26aa9:	0f 83 e1 fe ff ff    	jae    26990 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1cb0>
   26aaf:	90                   	nop
   26ab0:	66 0f 6e 44 24 08    	movd   0x8(%rsp),%xmm0
   26ab6:	e8 25 03 00 00       	call   26de0 <half::binary16::arch::x86::f32_to_f16_x86_f16c>
   26abb:	41 89 c6             	mov    %eax,%r14d
   26abe:	48 8b 44 24 28       	mov    0x28(%rsp),%rax
   26ac3:	4c 29 e8             	sub    %r13,%rax
   26ac6:	48 83 f8 01          	cmp    $0x1,%rax
   26aca:	76 22                	jbe    26aee <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1e0e>
   26acc:	66 46 89 74 2d 00    	mov    %r14w,0x0(%rbp,%r13,1)
   26ad2:	49 83 c5 02          	add    $0x2,%r13
   26ad6:	4c 89 6c 24 38       	mov    %r13,0x38(%rsp)
   26adb:	49 83 c7 03          	add    $0x3,%r15
   26adf:	49 83 ff 1b          	cmp    $0x1b,%r15
   26ae3:	0f 85 a7 fa ff ff    	jne    26590 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x18b0>
   26ae9:	e9 72 fa ff ff       	jmp    26560 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1880>
   26aee:	ba 02 00 00 00       	mov    $0x2,%edx
   26af3:	b9 01 00 00 00       	mov    $0x1,%ecx
   26af8:	41 b8 01 00 00 00    	mov    $0x1,%r8d
   26afe:	48 8d 7c 24 28       	lea    0x28(%rsp),%rdi
   26b03:	4c 89 ee             	mov    %r13,%rsi
   26b06:	e8 c5 05 00 00       	call   270d0 <<alloc::raw_vec::RawVecInner<_>>::reserve::do_reserve_and_handle::<alloc::alloc::Global>>
   26b0b:	48 8b 6c 24 30       	mov    0x30(%rsp),%rbp
   26b10:	4c 8b 6c 24 38       	mov    0x38(%rsp),%r13
   26b15:	eb b5                	jmp    26acc <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1dec>
   26b17:	48 8d 15 f2 67 04 00 	lea    0x467f2(%rip),%rdx        # 6d310 <anon.441e142b17a370d935234e81e4cac821.6.llvm.308828945136828694+0x180>
   26b1e:	ff 15 44 8f 04 00    	call   *0x48f44(%rip)        # 6fa68 <_DYNAMIC+0x3f8>
   26b24:	e9 a1 01 00 00       	jmp    26cca <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1fea>
   26b29:	48 8d 15 c8 67 04 00 	lea    0x467c8(%rip),%rdx        # 6d2f8 <anon.441e142b17a370d935234e81e4cac821.6.llvm.308828945136828694+0x168>
   26b30:	48 8b 74 24 50       	mov    0x50(%rsp),%rsi
   26b35:	ff 15 2d 8f 04 00    	call   *0x48f2d(%rip)        # 6fa68 <_DYNAMIC+0x3f8>
   26b3b:	e9 8a 01 00 00       	jmp    26cca <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1fea>
   26b40:	48 8d bc 24 a0 00 00 	lea    0xa0(%rsp),%rdi
   26b47:	00 
   26b48:	48 89 ee             	mov    %rbp,%rsi
   26b4b:	4c 89 ea             	mov    %r13,%rdx
   26b4e:	ff 15 04 91 04 00    	call   *0x49104(%rip)        # 6fc58 <_DYNAMIC+0x5e8>
   26b54:	83 bc 24 a0 00 00 00 	cmpl   $0x1,0xa0(%rsp)
   26b5b:	01 
   26b5c:	75 79                	jne    26bd7 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1ef7>
   26b5e:	48 8b 84 24 a8 00 00 	mov    0xa8(%rsp),%rax
   26b65:	00 
   26b66:	f2 0f 10 84 24 b0 00 	movsd  0xb0(%rsp),%xmm0
   26b6d:	00 00 
   26b6f:	f3 0f 10 8c 24 c0 00 	movss  0xc0(%rsp),%xmm1
   26b76:	00 00 
   26b78:	0f b6 8c 24 c4 00 00 	movzbl 0xc4(%rsp),%ecx
   26b7f:	00 
   26b80:	0f b6 94 24 c7 00 00 	movzbl 0xc7(%rsp),%edx
   26b87:	00 
   26b88:	48 8b b4 24 20 01 00 	mov    0x120(%rsp),%rsi
   26b8f:	00 
   26b90:	88 56 1f             	mov    %dl,0x1f(%rsi)
   26b93:	0f b7 94 24 c5 00 00 	movzwl 0xc5(%rsp),%edx
   26b9a:	00 
   26b9b:	66 89 56 1d          	mov    %dx,0x1d(%rsi)
   26b9f:	f2 0f 10 94 24 b8 00 	movsd  0xb8(%rsp),%xmm2
   26ba6:	00 00 
   26ba8:	48 89 06             	mov    %rax,(%rsi)
   26bab:	f2 0f 11 46 08       	movsd  %xmm0,0x8(%rsi)
   26bb0:	f2 0f 11 56 10       	movsd  %xmm2,0x10(%rsi)
   26bb5:	f3 0f 11 4e 18       	movss  %xmm1,0x18(%rsi)
   26bba:	88 4e 1c             	mov    %cl,0x1c(%rsi)
   26bbd:	48 8b 74 24 28       	mov    0x28(%rsp),%rsi
   26bc2:	48 85 f6             	test   %rsi,%rsi
   26bc5:	74 2d                	je     26bf4 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1f14>
   26bc7:	ba 01 00 00 00       	mov    $0x1,%edx
   26bcc:	48 89 ef             	mov    %rbp,%rdi
   26bcf:	ff 15 eb 8c 04 00    	call   *0x48ceb(%rip)        # 6f8c0 <_DYNAMIC+0x250>
   26bd5:	eb 1d                	jmp    26bf4 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1f14>
   26bd7:	48 8b 44 24 38       	mov    0x38(%rsp),%rax
   26bdc:	48 8b 8c 24 20 01 00 	mov    0x120(%rsp),%rcx
   26be3:	00 
   26be4:	48 89 41 18          	mov    %rax,0x18(%rcx)
   26be8:	0f 10 44 24 28       	movups 0x28(%rsp),%xmm0
   26bed:	0f 11 41 08          	movups %xmm0,0x8(%rcx)
   26bf1:	c6 01 ff             	movb   $0xff,(%rcx)
   26bf4:	48 8b b4 24 f0 00 00 	mov    0xf0(%rsp),%rsi
   26bfb:	00 
   26bfc:	48 85 f6             	test   %rsi,%rsi
   26bff:	74 17                	je     26c18 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1f38>
   26c01:	48 c1 e6 03          	shl    $0x3,%rsi
   26c05:	ba 08 00 00 00       	mov    $0x8,%edx
   26c0a:	48 8b bc 24 60 01 00 	mov    0x160(%rsp),%rdi
   26c11:	00 
   26c12:	ff 15 a8 8c 04 00    	call   *0x48ca8(%rip)        # 6f8c0 <_DYNAMIC+0x250>
   26c18:	48 8b 74 24 10       	mov    0x10(%rsp),%rsi
   26c1d:	48 85 f6             	test   %rsi,%rsi
   26c20:	74 14                	je     26c36 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1f56>
   26c22:	48 8b 7c 24 18       	mov    0x18(%rsp),%rdi
   26c27:	48 c1 e6 03          	shl    $0x3,%rsi
   26c2b:	ba 08 00 00 00       	mov    $0x8,%edx
   26c30:	ff 15 8a 8c 04 00    	call   *0x48c8a(%rip)        # 6f8c0 <_DYNAMIC+0x250>
   26c36:	be 00 8c 0a 00       	mov    $0xa8c00,%esi
   26c3b:	ba 08 00 00 00       	mov    $0x8,%edx
   26c40:	48 8b 7c 24 78       	mov    0x78(%rsp),%rdi
   26c45:	ff 15 75 8c 04 00    	call   *0x48c75(%rip)        # 6f8c0 <_DYNAMIC+0x250>
   26c4b:	48 8b bc 24 20 01 00 	mov    0x120(%rsp),%rdi
   26c52:	00 
   26c53:	e9 ec e1 ff ff       	jmp    24e44 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x164>
   26c58:	48 89 c7             	mov    %rax,%rdi
   26c5b:	48 8d 15 0e 67 04 00 	lea    0x4670e(%rip),%rdx        # 6d370 <anon.441e142b17a370d935234e81e4cac821.6.llvm.308828945136828694+0x1e0>
   26c62:	e9 0a ee ff ff       	jmp    25a71 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0xd91>
   26c67:	48 89 c7             	mov    %rax,%rdi
   26c6a:	48 8d 15 17 67 04 00 	lea    0x46717(%rip),%rdx        # 6d388 <anon.441e142b17a370d935234e81e4cac821.6.llvm.308828945136828694+0x1f8>
   26c71:	48 8b 9c 24 90 00 00 	mov    0x90(%rsp),%rbx
   26c78:	00 
   26c79:	e9 f3 ed ff ff       	jmp    25a71 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0xd91>
   26c7e:	48 8d 15 d3 66 04 00 	lea    0x466d3(%rip),%rdx        # 6d358 <anon.441e142b17a370d935234e81e4cac821.6.llvm.308828945136828694+0x1c8>
   26c85:	e9 e7 ed ff ff       	jmp    25a71 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0xd91>
   26c8a:	bf 08 00 00 00       	mov    $0x8,%edi
   26c8f:	be 50 00 00 00       	mov    $0x50,%esi
   26c94:	ff 15 3e 8c 04 00    	call   *0x48c3e(%rip)        # 6f8d8 <_DYNAMIC+0x268>
   26c9a:	eb 2e                	jmp    26cca <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1fea>
   26c9c:	4c 89 ef             	mov    %r13,%rdi
   26c9f:	4c 89 fe             	mov    %r15,%rsi
   26ca2:	ff 15 30 8c 04 00    	call   *0x48c30(%rip)        # 6f8d8 <_DYNAMIC+0x268>
   26ca8:	eb 20                	jmp    26cca <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x1fea>
   26caa:	bf 08 00 00 00       	mov    $0x8,%edi
   26caf:	be 00 8c 0a 00       	mov    $0xa8c00,%esi
   26cb4:	ff 15 1e 8c 04 00    	call   *0x48c1e(%rip)        # 6f8d8 <_DYNAMIC+0x268>
   26cba:	bf 08 00 00 00       	mov    $0x8,%edi
   26cbf:	be 00 2a 03 00       	mov    $0x32a00,%esi
   26cc4:	ff 15 0e 8c 04 00    	call   *0x48c0e(%rip)        # 6f8d8 <_DYNAMIC+0x268>
   26cca:	0f 0b                	ud2
   26ccc:	eb 2b                	jmp    26cf9 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x2019>
   26cce:	49 89 c4             	mov    %rax,%r12
   26cd1:	e9 d8 00 00 00       	jmp    26dae <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x20ce>
   26cd6:	e9 b2 00 00 00       	jmp    26d8d <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x20ad>
   26cdb:	49 89 c4             	mov    %rax,%r12
   26cde:	e9 82 00 00 00       	jmp    26d65 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x2085>
   26ce3:	e9 a5 00 00 00       	jmp    26d8d <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x20ad>
   26ce8:	49 89 c4             	mov    %rax,%r12
   26ceb:	4c 8b 74 24 08       	mov    0x8(%rsp),%r14
   26cf0:	eb 3d                	jmp    26d2f <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x204f>
   26cf2:	eb 5b                	jmp    26d4f <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x206f>
   26cf4:	49 89 c4             	mov    %rax,%r12
   26cf7:	eb 36                	jmp    26d2f <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x204f>
   26cf9:	49 89 c4             	mov    %rax,%r12
   26cfc:	48 8b 74 24 28       	mov    0x28(%rsp),%rsi
   26d01:	48 85 f6             	test   %rsi,%rsi
   26d04:	74 5f                	je     26d65 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x2085>
   26d06:	48 8b 7c 24 30       	mov    0x30(%rsp),%rdi
   26d0b:	ba 01 00 00 00       	mov    $0x1,%edx
   26d10:	eb 4d                	jmp    26d5f <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x207f>
   26d12:	49 89 c4             	mov    %rax,%r12
   26d15:	4d 85 ff             	test   %r15,%r15
   26d18:	74 15                	je     26d2f <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x204f>
   26d1a:	49 c1 e7 03          	shl    $0x3,%r15
   26d1e:	ba 08 00 00 00       	mov    $0x8,%edx
   26d23:	4c 89 ef             	mov    %r13,%rdi
   26d26:	4c 89 fe             	mov    %r15,%rsi
   26d29:	ff 15 91 8b 04 00    	call   *0x48b91(%rip)        # 6f8c0 <_DYNAMIC+0x250>
   26d2f:	48 8b 74 24 28       	mov    0x28(%rsp),%rsi
   26d34:	48 85 f6             	test   %rsi,%rsi
   26d37:	74 19                	je     26d52 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x2072>
   26d39:	48 8b 7c 24 30       	mov    0x30(%rsp),%rdi
   26d3e:	48 c1 e6 03          	shl    $0x3,%rsi
   26d42:	ba 08 00 00 00       	mov    $0x8,%edx
   26d47:	ff 15 73 8b 04 00    	call   *0x48b73(%rip)        # 6f8c0 <_DYNAMIC+0x250>
   26d4d:	eb 03                	jmp    26d52 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x2072>
   26d4f:	49 89 c4             	mov    %rax,%r12
   26d52:	be 20 03 00 00       	mov    $0x320,%esi
   26d57:	ba 08 00 00 00       	mov    $0x8,%edx
   26d5c:	4c 89 f7             	mov    %r14,%rdi
   26d5f:	ff 15 5b 8b 04 00    	call   *0x48b5b(%rip)        # 6f8c0 <_DYNAMIC+0x250>
   26d65:	48 8b b4 24 f0 00 00 	mov    0xf0(%rsp),%rsi
   26d6c:	00 
   26d6d:	48 85 f6             	test   %rsi,%rsi
   26d70:	74 1e                	je     26d90 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x20b0>
   26d72:	48 8b bc 24 f8 00 00 	mov    0xf8(%rsp),%rdi
   26d79:	00 
   26d7a:	48 c1 e6 03          	shl    $0x3,%rsi
   26d7e:	ba 08 00 00 00       	mov    $0x8,%edx
   26d83:	ff 15 37 8b 04 00    	call   *0x48b37(%rip)        # 6f8c0 <_DYNAMIC+0x250>
   26d89:	eb 05                	jmp    26d90 <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x20b0>
   26d8b:	eb 00                	jmp    26d8d <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x20ad>
   26d8d:	49 89 c4             	mov    %rax,%r12
   26d90:	48 8b 74 24 10       	mov    0x10(%rsp),%rsi
   26d95:	48 85 f6             	test   %rsi,%rsi
   26d98:	74 14                	je     26dae <xdremux_engine::apple_photographic_styles::apple_style_revert_local_data+0x20ce>
   26d9a:	48 8b 7c 24 18       	mov    0x18(%rsp),%rdi
   26d9f:	48 c1 e6 03          	shl    $0x3,%rsi
   26da3:	ba 08 00 00 00       	mov    $0x8,%edx
   26da8:	ff 15 12 8b 04 00    	call   *0x48b12(%rip)        # 6f8c0 <_DYNAMIC+0x250>
   26dae:	be 00 8c 0a 00       	mov    $0xa8c00,%esi
   26db3:	ba 08 00 00 00       	mov    $0x8,%edx
   26db8:	48 8b 7c 24 78       	mov    0x78(%rsp),%rdi
   26dbd:	ff 15 fd 8a 04 00    	call   *0x48afd(%rip)        # 6f8c0 <_DYNAMIC+0x250>
   26dc3:	4c 89 e7             	mov    %r12,%rdi
   26dc6:	e8 15 50 04 00       	call   6bde0 <_Unwind_Resume@plt>
   26dcb:	cc                   	int3
   26dcc:	cc                   	int3
   26dcd:	cc                   	int3
   26dce:	cc                   	int3
   26dcf:	cc                   	int3

0000000000026dd0 <half::binary16::arch::x86::f16_to_f32_x86_f16c>:
   26dd0:	c5 f9 6e c7          	vmovd  %edi,%xmm0
   26dd4:	c4 e2 79 13 c0       	vcvtph2ps %xmm0,%xmm0
   26dd9:	c3                   	ret
   26dda:	cc                   	int3
   26ddb:	cc                   	int3
   26ddc:	cc                   	int3
   26ddd:	cc                   	int3
   26dde:	cc                   	int3
   26ddf:	cc                   	int3

0000000000026de0 <half::binary16::arch::x86::f32_to_f16_x86_f16c>:
   26de0:	c5 f0 57 c9          	vxorps %xmm1,%xmm1,%xmm1
   26de4:	c4 e3 71 0c c0 01    	vblendps $0x1,%xmm0,%xmm1,%xmm0
   26dea:	c4 e3 79 1d c0 00    	vcvtps2ph $0x0,%xmm0,%xmm0
   26df0:	c5 f9 7e c0          	vmovd  %xmm0,%eax
   26df4:	c3                   	ret
   26df5:	cc                   	int3
   26df6:	cc                   	int3
   26df7:	cc                   	int3
   26df8:	cc                   	int3
   26df9:	cc                   	int3
   26dfa:	cc                   	int3
   26dfb:	cc                   	int3
   26dfc:	cc                   	int3
   26dfd:	cc                   	int3
   26dfe:	cc                   	int3
   26dff:	cc                   	int3

0000000000026e00 <core::ptr::drop_glue::<alloc::vec::Vec<alloc::vec::Vec<f64>>>>:
   26e00:	41 57                	push   %r15
   26e02:	41 56                	push   %r14
   26e04:	41 55                	push   %r13
   26e06:	41 54                	push   %r12
   26e08:	53                   	push   %rbx
   26e09:	49 89 fe             	mov    %rdi,%r14
   26e0c:	48 8b 5f 08          	mov    0x8(%rdi),%rbx
   26e10:	4c 8b 7f 10          	mov    0x10(%rdi),%r15
   26e14:	4d 85 ff             	test   %r15,%r15
   26e17:	74 3c                	je     26e55 <core::ptr::drop_glue::<alloc::vec::Vec<alloc::vec::Vec<f64>>>+0x55>
   26e19:	4c 8d 63 08          	lea    0x8(%rbx),%r12
   26e1d:	4c 8b 2d 9c 8a 04 00 	mov    0x48a9c(%rip),%r13        # 6f8c0 <_DYNAMIC+0x250>
   26e24:	eb 13                	jmp    26e39 <core::ptr::drop_glue::<alloc::vec::Vec<alloc::vec::Vec<f64>>>+0x39>
   26e26:	66 2e 0f 1f 84 00 00 	cs nopw 0x0(%rax,%rax,1)
   26e2d:	00 00 00 
   26e30:	49 83 c4 18          	add    $0x18,%r12
   26e34:	49 ff cf             	dec    %r15
   26e37:	74 1c                	je     26e55 <core::ptr::drop_glue::<alloc::vec::Vec<alloc::vec::Vec<f64>>>+0x55>
   26e39:	49 8b 74 24 f8       	mov    -0x8(%r12),%rsi
   26e3e:	48 85 f6             	test   %rsi,%rsi
   26e41:	74 ed                	je     26e30 <core::ptr::drop_glue::<alloc::vec::Vec<alloc::vec::Vec<f64>>>+0x30>
   26e43:	49 8b 3c 24          	mov    (%r12),%rdi
   26e47:	48 c1 e6 03          	shl    $0x3,%rsi
   26e4b:	ba 08 00 00 00       	mov    $0x8,%edx
   26e50:	41 ff d5             	call   *%r13
   26e53:	eb db                	jmp    26e30 <core::ptr::drop_glue::<alloc::vec::Vec<alloc::vec::Vec<f64>>>+0x30>
   26e55:	49 8b 06             	mov    (%r14),%rax
   26e58:	48 85 c0             	test   %rax,%rax
   26e5b:	74 1f                	je     26e7c <core::ptr::drop_glue::<alloc::vec::Vec<alloc::vec::Vec<f64>>>+0x7c>
   26e5d:	48 c1 e0 03          	shl    $0x3,%rax
   26e61:	48 8d 34 40          	lea    (%rax,%rax,2),%rsi
   26e65:	ba 08 00 00 00       	mov    $0x8,%edx
   26e6a:	48 89 df             	mov    %rbx,%rdi
   26e6d:	5b                   	pop    %rbx
   26e6e:	41 5c                	pop    %r12
   26e70:	41 5d                	pop    %r13
   26e72:	41 5e                	pop    %r14
   26e74:	41 5f                	pop    %r15
   26e76:	ff 25 44 8a 04 00    	jmp    *0x48a44(%rip)        # 6f8c0 <_DYNAMIC+0x250>
   26e7c:	5b                   	pop    %rbx
   26e7d:	41 5c                	pop    %r12
   26e7f:	41 5d                	pop    %r13
   26e81:	41 5e                	pop    %r14
   26e83:	41 5f                	pop    %r15
   26e85:	c3                   	ret
   26e86:	cc                   	int3
   26e87:	cc                   	int3
   26e88:	cc                   	int3
   26e89:	cc                   	int3
   26e8a:	cc                   	int3
   26e8b:	cc                   	int3
   26e8c:	cc                   	int3
   26e8d:	cc                   	int3
   26e8e:	cc                   	int3
   26e8f:	cc                   	int3

0000000000026e90 <<alloc::vec::Vec<alloc::vec::Vec<f64>> as alloc::vec::spec_from_iter_nested::SpecFromIterNested<alloc::vec::Vec<f64>, core::iter::adapters::map::Map<core::ops::range::Range<usize>, xdremux_engine::apple_photographic_styles::solve_linear_system::{closure#0}>>>::from_iter>:
   26e90:	41 57                	push   %r15
   26e92:	41 56                	push   %r14
   26e94:	41 55                	push   %r13
   26e96:	41 54                	push   %r12
   26e98:	53                   	push   %rbx
   26e99:	48 83 ec 30          	sub    $0x30,%rsp
   26e9d:	4c 8b 66 30          	mov    0x30(%rsi),%r12
   26ea1:	45 31 ff             	xor    %r15d,%r15d
   26ea4:	4c 2b 66 28          	sub    0x28(%rsi),%r12
   26ea8:	4d 0f 42 e7          	cmovb  %r15,%r12
   26eac:	4a 8d 04 e5 00 00 00 	lea    0x0(,%r12,8),%rax
   26eb3:	00 
   26eb4:	4c 8d 34 40          	lea    (%rax,%rax,2),%r14
   26eb8:	48 b8 55 55 55 55 55 	movabs $0x555555555555555,%rax
   26ebf:	55 55 05 
   26ec2:	49 39 c4             	cmp    %rax,%r12
   26ec5:	76 0c                	jbe    26ed3 <<alloc::vec::Vec<alloc::vec::Vec<f64>> as alloc::vec::spec_from_iter_nested::SpecFromIterNested<alloc::vec::Vec<f64>, core::iter::adapters::map::Map<core::ops::range::Range<usize>, xdremux_engine::apple_photographic_styles::solve_linear_system::{closure#0}>>>::from_iter+0x43>
   26ec7:	4c 89 ff             	mov    %r15,%rdi
   26eca:	4c 89 f6             	mov    %r14,%rsi
   26ecd:	ff 15 05 8a 04 00    	call   *0x48a05(%rip)        # 6f8d8 <_DYNAMIC+0x268>
   26ed3:	48 89 fb             	mov    %rdi,%rbx
   26ed6:	4d 85 f6             	test   %r14,%r14
   26ed9:	74 27                	je     26f02 <<alloc::vec::Vec<alloc::vec::Vec<f64>> as alloc::vec::spec_from_iter_nested::SpecFromIterNested<alloc::vec::Vec<f64>, core::iter::adapters::map::Map<core::ops::range::Range<usize>, xdremux_engine::apple_photographic_styles::solve_linear_system::{closure#0}>>>::from_iter+0x72>
   26edb:	49 89 f5             	mov    %rsi,%r13
   26ede:	ff 15 44 8a 04 00    	call   *0x48a44(%rip)        # 6f928 <_DYNAMIC+0x2b8>
   26ee4:	41 bf 08 00 00 00    	mov    $0x8,%r15d
   26eea:	be 08 00 00 00       	mov    $0x8,%esi
   26eef:	4c 89 f7             	mov    %r14,%rdi
   26ef2:	ff 15 38 8a 04 00    	call   *0x48a38(%rip)        # 6f930 <_DYNAMIC+0x2c0>
   26ef8:	48 85 c0             	test   %rax,%rax
   26efb:	74 ca                	je     26ec7 <<alloc::vec::Vec<alloc::vec::Vec<f64>> as alloc::vec::spec_from_iter_nested::SpecFromIterNested<alloc::vec::Vec<f64>, core::iter::adapters::map::Map<core::ops::range::Range<usize>, xdremux_engine::apple_photographic_styles::solve_linear_system::{closure#0}>>>::from_iter+0x37>
   26efd:	4c 89 ee             	mov    %r13,%rsi
   26f00:	eb 08                	jmp    26f0a <<alloc::vec::Vec<alloc::vec::Vec<f64>> as alloc::vec::spec_from_iter_nested::SpecFromIterNested<alloc::vec::Vec<f64>, core::iter::adapters::map::Map<core::ops::range::Range<usize>, xdremux_engine::apple_photographic_styles::solve_linear_system::{closure#0}>>>::from_iter+0x7a>
   26f02:	b8 08 00 00 00       	mov    $0x8,%eax
   26f07:	45 31 e4             	xor    %r12d,%r12d
   26f0a:	4c 89 24 24          	mov    %r12,(%rsp)
   26f0e:	48 89 44 24 08       	mov    %rax,0x8(%rsp)
   26f13:	48 8d 4c 24 10       	lea    0x10(%rsp),%rcx
   26f18:	48 c7 44 24 10 00 00 	movq   $0x0,0x10(%rsp)
   26f1f:	00 00 
   26f21:	48 89 44 24 28       	mov    %rax,0x28(%rsp)
   26f26:	48 89 4c 24 18       	mov    %rcx,0x18(%rsp)
   26f2b:	48 c7 44 24 20 00 00 	movq   $0x0,0x20(%rsp)
   26f32:	00 00 
   26f34:	48 8d 44 24 18       	lea    0x18(%rsp),%rax
   26f39:	48 89 f7             	mov    %rsi,%rdi
   26f3c:	48 89 c6             	mov    %rax,%rsi
   26f3f:	e8 ac 03 00 00       	call   272f0 <<core::iter::adapters::map::Map<core::ops::range::Range<usize>, xdremux_engine::apple_photographic_styles::solve_linear_system::{closure#0}> as core::iter::traits::iterator::Iterator>::fold::<(), core::iter::traits::iterator::Iterator::for_each::call<alloc::vec::Vec<f64>, <alloc::vec::Vec<alloc::vec::Vec<f64>>>::extend_trusted<core::iter::adapters::map::Map<core::ops::range::Range<usize>, xdremux_engine::apple_photographic_styles::solve_linear_system::{closure#0}>>::{closure#0}>::{closure#0}>>
   26f44:	48 8b 44 24 10       	mov    0x10(%rsp),%rax
   26f49:	48 89 43 10          	mov    %rax,0x10(%rbx)
   26f4d:	0f 10 04 24          	movups (%rsp),%xmm0
   26f51:	0f 11 03             	movups %xmm0,(%rbx)
   26f54:	48 89 d8             	mov    %rbx,%rax
   26f57:	48 83 c4 30          	add    $0x30,%rsp
   26f5b:	5b                   	pop    %rbx
   26f5c:	41 5c                	pop    %r12
   26f5e:	41 5d                	pop    %r13
   26f60:	41 5e                	pop    %r14
   26f62:	41 5f                	pop    %r15
   26f64:	c3                   	ret
   26f65:	48 89 c3             	mov    %rax,%rbx
   26f68:	48 89 e7             	mov    %rsp,%rdi
   26f6b:	e8 90 fe ff ff       	call   26e00 <core::ptr::drop_glue::<alloc::vec::Vec<alloc::vec::Vec<f64>>>>
   26f70:	48 89 df             	mov    %rbx,%rdi
   26f73:	e8 68 4e 04 00       	call   6bde0 <_Unwind_Resume@plt>
   26f78:	cc                   	int3
   26f79:	cc                   	int3
   26f7a:	cc                   	int3
   26f7b:	cc                   	int3
   26f7c:	cc                   	int3
   26f7d:	cc                   	int3
   26f7e:	cc                   	int3
   26f7f:	cc                   	int3
