#include "math.h"

int power(int a, int b) {
	int acc = 1;
	for (int i = 0; i < b; i++) {
		acc *= a;
	}
	return a;
}
