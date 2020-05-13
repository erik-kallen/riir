#include <tvm/tvm_memory.h>

#include <stdlib.h>
#include <string.h>

#define NUM_REGISTERS 17

struct tvm_mem *tvm_mem_create(size_t size)
{
	struct tvm_mem *m =
		(struct tvm_mem *)calloc(1, sizeof(struct tvm_mem));

	m->registers = calloc(NUM_REGISTERS, sizeof(union tvm_reg_u));

	m->mem_space_size = size;
	m->mem_space = (int *)calloc(size, 1);

	return m;
}

void tvm_mem_destroy(struct tvm_mem *m)
{
	free(m->mem_space);
	free(m->registers);
	free(m);
}

void tvm_stack_create(struct tvm_mem *mem, size_t size)
{
	mem->registers[0x7].i32_ptr =
		((int32_t *)mem->mem_space) + (size / sizeof(int32_t));
	mem->registers[0x6].i32_ptr = mem->registers[0x7].i32_ptr;
}

void tvm_stack_push(struct tvm_mem *mem, int *item)
{
	mem->registers[0x6].i32_ptr -= 1;
	*mem->registers[0x6].i32_ptr = *item;
}

void tvm_stack_pop(struct tvm_mem *mem, int *dest)
{
	*dest = *mem->registers[0x6].i32_ptr;
	mem->registers[0x6].i32_ptr += 1;
}
