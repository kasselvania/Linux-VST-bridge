; Original Linux x86-64 syscall leaf, entered only by Wine's Unix dispatcher.
; SysV input RDI points to the POD in linux_wait.h. No CRT, Win32, TLS or calls.
; Only RBX (callee-saved) is used; Linux syscall destroys RCX/R11.
.code
PUBLIC lvb_linux_wait_leaf
lvb_linux_wait_leaf PROC
    db 0f3h, 00fh, 01eh, 0fah ; ENDBR64 for an indirect Unix call
    push rbx
    mov rbx, rdi
    mov eax, DWORD PTR [rbx+28]
    cmp eax, 0
    je monotonic
    cmp eax, 1
    je wait_word
    cmp eax, 2
    je wake_word
    mov rax, -22
    jmp finish
monotonic:
    mov eax, 228 ; __NR_clock_gettime
    mov edi, 1 ; CLOCK_MONOTONIC
    lea rsi, [rbx+8]
    syscall
    jmp finish
wait_word:
    mov eax, 202 ; __NR_futex
    mov rdi, QWORD PTR [rbx]
    mov esi, 9 ; FUTEX_WAIT_BITSET, shared, CLOCK_MONOTONIC absolute time
    mov edx, DWORD PTR [rbx+24]
    lea r10, [rbx+8]
    xor r8d, r8d
    mov r9d, 0ffffffffh ; FUTEX_BITSET_MATCH_ANY
    syscall
    jmp finish
wake_word:
    mov eax, 202
    mov rdi, QWORD PTR [rbx]
    mov esi, 1 ; FUTEX_WAKE, shared
    mov edx, 1
    xor r10d, r10d
    xor r8d, r8d
    xor r9d, r9d
    syscall
finish:
    mov QWORD PTR [rbx+32], rax
    pop rbx
    xor eax, eax ; Unix-call NTSTATUS success; raw Linux result is in POD
    ret
lvb_linux_wait_leaf ENDP
END
