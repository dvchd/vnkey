CXX = g++
CXXFLAGS = -std=c++17 -O0 -g
ENGINE_DIR = ../../vnkey-engine
INCLUDES = -I../../vnkey-fcitx5/src
ENGINE_LIB = $(ENGINE_DIR)/target/release/libvnkey_engine.a
LIBS = -lpthread -ldl -lm

.PHONY: all test clean

all: test_before_fix

test_before_fix: test_before_fix.cpp $(ENGINE_LIB)
	$(CXX) $(CXXFLAGS) -o $@ $< $(INCLUDES) $(ENGINE_LIB) $(LIBS)

test: test_before_fix
	./test_before_fix

clean:
	rm -f test_before_fix
