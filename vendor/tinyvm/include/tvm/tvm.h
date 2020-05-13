#ifndef TVM_H_
#define TVM_H_

#include <stddef.h>

#include "tvm_file.h"
#include "tvm_preprocessor.h"
#include "tvm_stack.h"

#include "tvm_memory.h"
#include "tvm_program.h"
#include "tvm_tokens.h"

struct tvm_ctx {
	struct tvm_prog *prog;
	struct tvm_mem *mem;
};

struct tvm_ctx *tvm_vm_create();
void tvm_vm_destroy(struct tvm_ctx *vm);

int tvm_vm_interpret(struct tvm_ctx *vm, char *filename);
void tvm_vm_run(struct tvm_ctx *vm);

void tvm_step(struct tvm_ctx *vm, int *instr_idx);

#endif
