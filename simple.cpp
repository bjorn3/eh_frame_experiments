// clang -c simple.cpp -o simple.o -ffunction-sections

#include <cstdio>

int my_throw();

class PrintOnDrop {
public:
    PrintOnDrop() {}
    ~PrintOnDrop() {
        printf("Drop");
    }
};

/*
int cleanup() {
    PrintOnDrop foo;
    my_throw();
}
*/

/*
int my_catch() {
    try {
        my_throw();
    } catch(int *exc) {}
}
*/

/*
int catch_all() {
    try {
        my_throw();
    } catch(...) {}
}
*/

/*
int exception_spec() throw(char*, int) {
    my_throw();
}
*/

/*
int exception_spec_and_catch() throw(char*, int) {
    try {
        my_throw();
    } catch(short *exc) {}
}
*/

int exception_spec_and_catch_and_cleanup() throw(char*, int) {
    PrintOnDrop foo;
    try {
        my_throw();
    } catch(short *exc) {}
}
