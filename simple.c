#include <stdio.h>

char prog[64];
char band[64];

char read_prog() {
    char len = 0;
    char input = getchar();
    while (input != '#') {
        prog[len] = input;
        len = len + 1;
        input = getchar();
    }
    return len;
}

char move_front(char ip) {
    char v = prog[ip];
    while (v != ']') {
        ip = ip + 1;
        v = prog[ip];
    }
    return ip;
}

char move_back(char ip) {
    char v = prog[ip];
    while (v != '[') {
        ip = ip - 1;
        v = prog[ip];
    }
    return ip;
}

void run_prog(char len) {
    char ip = 0;
    char pos = 0;
    char v;

    while (ip != len) {
        char c = prog[ip];
        if (c == '.') {
            v = band[pos];
            putchar(v);
        }
        if (c == ',') {
            v = getchar();
            band[pos] = v;
        }
        if (c == '+') {
            v = band[pos];
            band[pos] = v + 1;
        }
        if (c == '-') {
            v = band[pos];
            band[pos] = v - 1;
        }
        if (c == '<') {
            pos = pos - 1;
            if(pos == 255) {
                pos = 63;
            } 
        }
        if (c == '>') {
            pos = pos + 1;
            if(pos == 64) {
                pos = 0;
            } 
        }
        if (c == '[') {
            v = band[pos];
            if (v == 0) {
                ip = move_front(ip);
            }
        }
        if (c == ']') {
            v = band[pos];
            if (v != 0) {
                ip = move_back(ip);
            }
        }
        ip = ip + 1;
    }
    return;
}

void main() {
    char len = read_prog();

    run_prog(len);
    return;
}
