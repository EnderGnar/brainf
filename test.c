char print(char v) {


    putchar(v);
    return v;
}


void print_num(char num) {
    char x = 48;

    while (num != 0) {
        x = x + 1;
        num = num - 1;
    }

    putchar(x);
    return;
}

char main() {
    char v = 0;
    char xxx = 0;
    v = getchar();

    while (v != 3) {
        xxx = xxx + 1;
        putchar(xxx);
        print_num(v);
        v = getchar();
    }
    return;
}
