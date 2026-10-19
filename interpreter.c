#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#define BAND_LENGTH (1024)

int find_end(char *program, int ip, int program_len) {
    int depth = 0;

    for (int i = ip + 1; i < program_len; i++) {
        if (program[i] == '[') depth += 1;
        if (program[i] == ']') {
            if (depth == 0) return i;
            depth -= 1;
        }
    }

    return -1;
}

int find_start(char *program, int ip) {
    int depth = 0;

    for (int i = ip - 1; i >= 0; i--) {
        if (program[i] == ']') depth += 1;
        if (program[i] == '[') {
            if (depth == 0) return i;
            depth -= 1;
        }
    }

    return -1;
}

void interpret(char *program, int decimal_mode) {
    int ip = 0;
    int pos = 0;

    unsigned char *band = (unsigned char *) calloc(BAND_LENGTH, 1);
    
    int program_len = strlen(program);
    int *jump_list = (int *) malloc(program_len);
    for(int i = 0; i < strlen(program); i++) jump_list[i] = -1;

    while (ip < program_len) {
        int in;
        switch (program[ip])
        {
        case '+':
            band[pos]++;
            break;
        case '-':
            band[pos]--;
            break;
        case '>':
            pos = (pos + 1) % BAND_LENGTH;
            break;
        case '<':
            pos = (pos - 1 + BAND_LENGTH) % BAND_LENGTH;
            break;
        case '.':
            if (decimal_mode) {
                printf("%d ", band[pos]);
            } else {
                putc(band[pos], stdout);
            }
            break;
        case ',':
            if (decimal_mode) {
                scanf("%d", &in);
            } else {
                in = getc(stdin);
            }

            if (in < 0) return;
            band[pos] = in;
            break;
        case '[':
            // Only jump on zero
            if (band[pos] != 0) break;
            if (jump_list[ip] < 0) {
                jump_list[ip] = find_end(program, ip, program_len);
                if(jump_list[ip] == -1) {
                    fprintf(stderr, "Unmatched [ at %d\n", ip);
                    return;
                }
            }
            ip = jump_list[ip];
            break;
        case ']':
            // Only jump on non-zero
            if (band[pos] == 0) break;
            if (jump_list[ip] < 0) {
                jump_list[ip] = find_start(program, ip);
                if(jump_list[ip] == -1) {
                    fprintf(stderr, "Unmatched ] at %d\n", ip);
                    return;
                }
            }
            ip = jump_list[ip];
            break;
        }

        ip += 1;
    }

}

int main(int argc, char **argv) {
    if(argc != 3) {
        fprintf(stderr, "run with %s <program> <decimal_mode 0/1>", argv[0]);
        return 1;
    }

    FILE *prog_file = fopen(argv[1], "r");
    fseek(prog_file, 0, SEEK_END);
    int program_length = ftell(prog_file);
    rewind(prog_file);

    if(program_length <= 0) {
        fprintf(stderr, "File read failed.\n");
    }

    char *program = malloc(program_length);
    fread(program, 1, program_length, prog_file);

    if (program[program_length-1] == '\n') program[program_length-1] = 0;

    int decimal_mode = atoi(argv[2]);

    interpret(program, decimal_mode);
    printf("\n");
    return 0;
}
