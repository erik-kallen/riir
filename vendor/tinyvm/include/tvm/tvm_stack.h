#ifndef TVM_STACK_H_
#define TVM_STACK_H_

#define MIN_STACK_SIZE (2 * 1024 * 1024) /* 2 MB */

#include "tvm_memory.h"

/* Initialize our stack by setting the base pointer and stack pointer */

void tvm_stack_create(struct tvm_mem *mem, size_t size);

void tvm_stack_push(struct tvm_mem *mem, int *item);

void tvm_stack_pop(struct tvm_mem *mem, int *dest);

#endif
