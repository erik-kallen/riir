#include <tvm/tvm.h>

#include <tvm/tvm_lexer.h>
#include <tvm/tvm_parser.h>

struct tvm_ctx *tvm_vm_create()
{
	struct tvm_ctx *vm =
		(struct tvm_ctx *)calloc(1, sizeof(struct tvm_ctx));

	if (!vm)
		return NULL;
	vm->mem = tvm_mem_create(MIN_MEMORY_SIZE);
	vm->prog = tvm_prog_create();

	if (!vm->mem || !vm->prog) {
		tvm_vm_destroy(vm);
		return NULL;
	}

	tvm_stack_create(vm->mem, MIN_STACK_SIZE);
	return vm;
}

int tvm_vm_interpret(struct tvm_ctx *vm, char *filename)
{
	FILE *filp = NULL;
	int source_length = 0;

	/* Attempt to open the file.
	 * If the file cannot be opened, try once more.
	 */
	if (filename) {
		for (int i = 0; i < 2; i++) {
			if (!filp)
				filp = tvm_fopen(filename, ".vm", "r");
		}
	}

	if (!filp) {
		printf("File was not found, or does not exist. Unable to interpret.\n");
		return 1;
	}

	source_length = tvm_flength(filp);
	char *source = calloc(source_length, sizeof(char));

	tvm_fcopy(source, source_length, filp);
	fclose(filp);

	int err = tvm_preprocess(&source, &source_length, vm->prog->defines);

	/* The preprocessor encountered a problem. */
	if (err < 0)
		return 1;

	struct tvm_lexer_ctx *lexer_ctx = lexer_create();

	tvm_lex(lexer_ctx, source, vm->prog->defines);
	free(source);

	if (tvm_parse_labels(vm, (const char ***)lexer_ctx->tokens) != 0)
		return 1;
	if (tvm_parse_program(vm, (const char ***)lexer_ctx->tokens) != 0)
		return 1;

	tvm_lexer_destroy(lexer_ctx);
	return 0;
}

void tvm_vm_run(struct tvm_ctx *vm)
{
	int *instr_idx = &vm->mem->registers[0x8].i32;
	*instr_idx = vm->prog->start;

	for (; vm->prog->instr[*instr_idx] != -0x1; ++(*instr_idx))
		tvm_step(vm, instr_idx);
}


void tvm_vm_destroy(struct tvm_ctx *vm)
{
	if (vm && vm->mem)
		tvm_mem_destroy(vm->mem);
	if (vm && vm->prog)
		tvm_prog_destroy(vm->prog);
	if (vm)
		free(vm);
}

void tvm_step(struct tvm_ctx *vm, int *instr_idx)
{
	int **args = vm->prog->args[*instr_idx];

	switch (vm->prog->instr[*instr_idx]) {
/* nop   */	case 0x0:  break;
/* int   */	case 0x1:  /* unimplemented */ break;
/* mov   */	case 0x2:  *args[0] = *args[1]; break;
/* push  */	case 0x3:  tvm_stack_push(vm->mem, args[0]); break;
/* pop   */	case 0x4:  tvm_stack_pop(vm->mem, args[0]); break;
/* pushf */	case 0x5:  tvm_stack_push(vm->mem, &vm->mem->FLAGS); break;
/* popf  */	case 0x6:  tvm_stack_pop(vm->mem, args[0]); break;
/* inc   */	case 0x7:  ++(*args[0]); break;
/* dec   */	case 0x8:  --(*args[0]); break;
/* add   */	case 0x9:  *args[0] += *args[1]; break;
/* sub   */	case 0xA:  *args[0] -= *args[1]; break;
/* mul   */	case 0xB:  *args[0] *= *args[1]; break;
/* div   */	case 0xC:  *args[0] /= *args[1]; break;
/* mod   */	case 0xD:  vm->mem->remainder = *args[0] % *args[1]; break;
/* rem   */	case 0xE:  *args[0] = vm->mem->remainder; break;
/* not   */	case 0xF:  *args[0] = ~(*args[0]); break;
/* xor   */	case 0x10:  *args[0] ^= *args[1];  break;
/* or    */	case 0x11: *args[0] |= *args[1];   break;
/* and   */	case 0x12: *args[0] &= *args[1];   break;
/* shl   */	case 0x13: *args[0] <<= *args[1];  break;
/* shr   */	case 0x14: *args[0] >>= *args[1];  break;
/* cmp   */	case 0x15: vm->mem->FLAGS =
				((*args[0] == *args[1]) | (*args[0] > *args[1]) << 1);
				break;
/* call	 */	case 0x17: tvm_stack_push(vm->mem, instr_idx);
/* jmp	 */	case 0x16: *instr_idx = *args[0] - 1; break;
/* ret   */	case 0x18: tvm_stack_pop(vm->mem, instr_idx);
				break;
/* je    */	case 0x19:
				*instr_idx = (vm->mem->FLAGS & 0x1)
					? *args[0] - 1 : *instr_idx;
				break;
/* jne   */	case 0x1A:
				*instr_idx = (!(vm->mem->FLAGS & 0x1))
					? *args[0] - 1 : *instr_idx;
				break;
/* jg    */	case 0x1B:
				*instr_idx = (vm->mem->FLAGS & 0x2)
					? *args[0] - 1 : *instr_idx;
				break;
/* jge   */	case 0x1C:
				*instr_idx = (vm->mem->FLAGS & 0x3)
					? *args[0] - 1 : *instr_idx;
				break;
/* jl    */	case 0x1D:
				*instr_idx = (!(vm->mem->FLAGS & 0x3))
					? *args[0] - 1 : *instr_idx;
				break;
/* jle   */	case 0x1E:
				*instr_idx = (!(vm->mem->FLAGS & 0x2))
					? *args[0] - 1 : *instr_idx;
				break;
/* prn   */	case 0x1F: printf("%i\n", *args[0]);
	};
}
