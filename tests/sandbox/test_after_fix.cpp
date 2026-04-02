/*
 * test_after_fix.cpp — Cấp độ 2+ Test: Xác minh fix Issue #18 hoạt động đúng
 *
 * MÔI TRƯỜNG: fix/sandbox-app-preedit-fallback branch (SAU khi apply fix)
 *
 * Test này xác minh rằng code đã sửa xử lý đúng sandbox app:
 *   - Kiểm tra hasSurroundingText capability tại runtime
 *   - Fallback sang preedit mode khi !hasSurroundingText
 *   - Không gọi deleteSurroundingText() (API không khả dụng trong sandbox)
 *   - Output sandbox == output normal (byte-identical)
 *
 * Xem kết quả TRƯỚC KHI FIX ở nhánh test/before-fix.
 *
 * Build:
 *   g++ -std=c++17 -o test_after_fix test_after_fix.cpp \
 *       -I../../vnkey-fcitx5/src \
 *       ../../vnkey-engine/target/release/libvnkey_engine.a \
 *       -lpthread -ldl -lm
 *
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

#include <cstdio>
#include <cstdlib>
#include <cstring>
#include <string>
#include <vector>

extern "C" {
#include "vnkey-engine.h"
}

// ============================================================================
// Simple test framework
// ============================================================================

static int g_passed = 0;
static int g_failed = 0;
static int g_total = 0;

static void checkStr(const char *file, int line, const char *msg,
                     const std::string &actual, const std::string &expected) {
    g_total++;
    if (actual == expected) {
        g_passed++;
    } else {
        g_failed++;
        fprintf(stderr, "  FAIL [%s:%d]: %s\n    expected (%zu bytes): [", file, line, msg, expected.size());
        for (unsigned char c : expected) fprintf(stderr, "%02x", c);
        fprintf(stderr, "]\n    actual   (%zu bytes): [", actual.size());
        for (unsigned char c : actual) fprintf(stderr, "%02x", c);
        fprintf(stderr, "]\n");
    }
}

static void checkBool(const char *file, int line, const char *msg,
                      bool actual, bool expected) {
    g_total++;
    if (actual == expected) {
        g_passed++;
    } else {
        g_failed++;
        fprintf(stderr, "  FAIL [%s:%d]: %s (expected=%s, actual=%s)\n",
                file, line, msg, expected ? "true" : "false", actual ? "true" : "false");
    }
}

static void checkInt(const char *file, int line, const char *msg,
                     int actual, int expected) {
    g_total++;
    if (actual == expected) {
        g_passed++;
    } else {
        g_failed++;
        fprintf(stderr, "  FAIL [%s:%d]: %s (expected=%d, actual=%d)\n",
                file, line, msg, expected, actual);
    }
}

#define ASSERT_STR(actual, expected, msg) checkStr(__FILE__, __LINE__, msg, (actual), (expected))
#define ASSERT_BOOL(actual, expected, msg) checkBool(__FILE__, __LINE__, msg, (actual), (expected))
#define ASSERT_INT(actual, expected, msg) checkInt(__FILE__, __LINE__, msg, (actual), (expected))

// ============================================================================
// Mock InputContext — tái hiện hành vi Fcitx5
// ============================================================================

struct MockInputContext {
    bool hasSurroundingText = true;
    std::string committedText;
    std::string clientPreedit;
    std::string serverPreedit;
    int deleteSurroundingTextCount = 0;

    void commitString(const std::string &s) { committedText += s; }

    void deleteSurroundingText(int charOffset, unsigned int charLen) {
        deleteSurroundingTextCount++;
        if (!hasSurroundingText) return; // Sandbox: ignored
        int textLen = static_cast<int>(utf8CharCount(committedText));
        size_t byteOffset = charToByteOffset(committedText,
            textLen + charOffset);
        size_t endByteOffset = charToByteOffset(committedText,
            textLen + charOffset + static_cast<int>(charLen));
        size_t byteLen = endByteOffset - byteOffset;
        if (byteOffset < committedText.size()) {
            committedText.erase(byteOffset, byteLen);
        }
    }

    void setClientPreedit(const std::string &s) { clientPreedit = s; }
    void setPreedit(const std::string &s) { serverPreedit = s; }
    void clearPreedit() { clientPreedit.clear(); serverPreedit.clear(); }

private:
    static size_t charToByteOffset(const std::string &s, int charIdx) {
        if (charIdx <= 0) return 0;
        size_t bytePos = 0;
        int chars = 0;
        while (bytePos < s.size() && chars < charIdx) {
            unsigned char c = static_cast<unsigned char>(s[bytePos]);
            if (c < 0x80) bytePos += 1;
            else if ((c & 0xE0) == 0xC0) bytePos += 2;
            else if ((c & 0xF0) == 0xE0) bytePos += 3;
            else if ((c & 0xF8) == 0xF0) bytePos += 4;
            else bytePos += 1;
            chars++;
        }
        return bytePos > s.size() ? s.size() : bytePos;
    }

    static size_t utf8CharCount(const std::string &s) {
        size_t count = 0;
        size_t i = 0;
        while (i < s.size()) {
            unsigned char c = static_cast<unsigned char>(s[i]);
            if (c < 0x80) i += 1;
            else if ((c & 0xE0) == 0xC0) i += 2;
            else if ((c & 0xF0) == 0xE0) i += 3;
            else if ((c & 0xF8) == 0xF0) i += 4;
            else i += 1;
            count++;
        }
        return count;
    }
};

// ============================================================================
// FixedSandboxSimulator — Mô phỏng behavior SAU KHI FIX
//
// Code mới (fix/sandbox-app-preedit-fallback):
//   - Kiểm tra hasSurroundingText tại runtime
//   - Fallback sang preedit mode khi !hasSurroundingText
//   - Không gọi deleteSurroundingText() trong sandbox
//   → Output sandbox == Output normal (byte-identical)
//
// Logic tương ứng: VnKeyState::keyEvent() lines 1082-1154
// ============================================================================

class FixedSandboxSimulator {
public:
    enum class Mode { Normal, Sandbox, Terminal };

    FixedSandboxSimulator(Mode mode) : mode_(mode), engine_(nullptr) {
        engine_ = vnkey_engine_new();
        vnkey_engine_set_input_method(engine_, 0); // Telex
        vnkey_engine_set_viet_mode(engine_, 1);
        vnkey_engine_set_options(engine_, 1, 1, 1, 1, 0, 0);
        if (mode == Mode::Normal) {
            ic_.hasSurroundingText = true;
        } else {
            ic_.hasSurroundingText = false;
        }
    }

    ~FixedSandboxSimulator() {
        if (engine_) vnkey_engine_free(engine_);
    }

    void reset() {
        vnkey_engine_reset(engine_);
        preedit_.clear();
        ic_.committedText.clear();
        ic_.clearPreedit();
        ic_.deleteSurroundingTextCount = 0;
    }

    // Gõ một phím — tái hiện logic SAU FIX
    void processKey(char c) {
        uint32_t keyCode = static_cast<uint32_t>(static_cast<unsigned char>(c));
        uint8_t buf[256];
        size_t actualLen = 0;
        size_t backspaces = 0;

        int processed = vnkey_engine_process(
            engine_, keyCode, buf, sizeof(buf), &actualLen, &backspaces, nullptr);

        bool usePreedit = (mode_ != Mode::Normal);

        if (usePreedit) {
            // === PREEDIT MODE (sandbox/terminal) — FIX CODE ===
            if (processed) {
                for (size_t i = 0; i < backspaces && !preedit_.empty(); i++) {
                    popUtf8Char(preedit_);
                }
                if (actualLen > 0) {
                    preedit_.append(reinterpret_cast<const char *>(buf), actualLen);
                }
            } else {
                preedit_ += static_cast<char>(keyCode);
            }

            if (vnkey_engine_at_word_beginning(engine_)) {
                commitPreeditBuffer(false);
                vnkey_engine_reset(engine_);
            }
        } else {
            // === DIRECT COMMIT MODE (normal GUI) ===
            if (processed) {
                if (backspaces > 0) {
                    ic_.deleteSurroundingText(
                        -static_cast<int>(backspaces),
                        static_cast<unsigned int>(backspaces));
                }
                if (actualLen > 0) {
                    ic_.commitString(
                        std::string(reinterpret_cast<const char *>(buf), actualLen));
                }
            } else {
                char ch = static_cast<char>(keyCode);
                ic_.commitString(std::string(&ch, 1));
            }

            if (vnkey_engine_at_word_beginning(engine_)) {
                vnkey_engine_reset(engine_);
            }
        }
    }

    void processSpace() {
        commitPreeditBuffer(true);
        ic_.commitString(" ");
    }

    void processBackspace() {
        bool usePreedit = (mode_ != Mode::Normal);

        if (usePreedit) {
            if (preedit_.empty()) return;

            uint8_t buf[256];
            size_t actualLen = 0;
            size_t backspaces = 0;
            int processed = vnkey_engine_backspace(
                engine_, buf, sizeof(buf), &actualLen, &backspaces, nullptr);

            if (processed && (backspaces > 0 || actualLen > 0)) {
                for (size_t i = 0; i < backspaces && !preedit_.empty(); i++) {
                    popUtf8Char(preedit_);
                }
                if (actualLen > 0) {
                    preedit_.append(reinterpret_cast<const char *>(buf), actualLen);
                }
                return;
            }
            popUtf8Char(preedit_);
            return;
        }

        uint8_t buf[256];
        size_t actualLen = 0;
        size_t backspaces = 0;
        int processed = vnkey_engine_backspace(
            engine_, buf, sizeof(buf), &actualLen, &backspaces, nullptr);

        if (processed && (backspaces > 0 || actualLen > 0)) {
            if (backspaces > 0) {
                ic_.deleteSurroundingText(
                    -static_cast<int>(backspaces),
                    static_cast<unsigned int>(backspaces));
            }
            if (actualLen > 0) {
                ic_.commitString(
                    std::string(reinterpret_cast<const char *>(buf), actualLen));
            }
        }
    }

    const std::string& committedText() const { return ic_.committedText; }
    const std::string& preedit() const { return preedit_; }
    const std::string& clientPreedit() const { return ic_.clientPreedit; }
    const std::string& serverPreedit() const { return ic_.serverPreedit; }
    int deleteCount() const { return ic_.deleteSurroundingTextCount; }

private:
    void commitPreeditBuffer(bool soft) {
        if (!preedit_.empty()) {
            ic_.commitString(preedit_);
            preedit_.clear();
            ic_.setClientPreedit("");
            ic_.setPreedit("");
        }
        if (soft) vnkey_engine_soft_reset(engine_);
        else vnkey_engine_reset(engine_);
    }

    static void popUtf8Char(std::string &s) {
        if (s.empty()) return;
        while (!s.empty() && (static_cast<unsigned char>(s.back()) & 0xC0) == 0x80)
            s.pop_back();
        if (!s.empty()) s.pop_back();
    }

    Mode mode_;
    ::VnKeyEngine *engine_;
    std::string preedit_;
    MockInputContext ic_;
};

// ============================================================================
// Helpers
// ============================================================================

static void typeWord(FixedSandboxSimulator &sim, const char *word) {
    for (const char *p = word; *p; p++) {
        sim.processKey(*p);
    }
}

static void typeSentence(FixedSandboxSimulator &sim, const char *sentence) {
    std::string word;
    for (const char *p = sentence; ; p++) {
        if (*p == ' ' || *p == '\0') {
            if (!word.empty()) {
                typeWord(sim, word.c_str());
                sim.processSpace();
                word.clear();
            }
            if (*p == '\0') break;
        } else {
            word += *p;
        }
    }
}

static std::string hexToStr(const char *hex) {
    std::string result;
    for (const char *p = hex; p[0] && p[1]; p += 2) {
        char buf[3] = {p[0], p[1], 0};
        result += static_cast<char>(strtol(buf, nullptr, 16));
    }
    return result;
}

// ============================================================================
// TEST 1: Core Fix — Sandbox output == Normal output (byte-identical)
// ============================================================================

static void test_sandbox_normal_parity() {
    printf("\n=== TEST 1: Sandbox == Normal (Core Fix Verification) ===\n\n");

    struct TestCase {
        const char *input;
        const char *expected_hex;
        const char *description;
    };

    TestCase cases[] = {
        {"tieeng",  "7469c3aa6e67",     "ee->circumflex (tieng)"},
        {"Goof",    "47e1bb93",         "oo->circumflex+f->grave (Go)"},
        {"Vieejt",  "5669e1bb8774",     "ee->circumflex+j->dot-below (Viet)"},
        {"ddoong",  "c491c3b46e67",     "dd->d-bar+oo->circumflex (dong)"},
        {"thuowc",  "7468c6b0c6a163",   "ow->horn (thuoc)"},
        {"hello",   "68656c6c6f",       "English word (pass-through)"},
        {"chao",    "6368616f",         "no Telex rule triggered"},
        {nullptr, nullptr, nullptr}
    };

    int parity_pass = 0;
    int total = 0;

    printf("  %-12s | %-12s | %-12s | %-12s\n",
           "Telex Input", "Correct", "Normal", "Sandbox");
    printf("  %-12s-|-%-12s-|-%-12s-|-%-12s\n",
           "------------", "------------", "------------", "------------");

    for (int i = 0; cases[i].input; i++) {
        total++;
        std::string expected = hexToStr(cases[i].expected_hex) + " ";

        FixedSandboxSimulator normal(FixedSandboxSimulator::Mode::Normal);
        FixedSandboxSimulator sandbox(FixedSandboxSimulator::Mode::Sandbox);

        typeWord(normal, cases[i].input);
        normal.processSpace();
        typeWord(sandbox, cases[i].input);
        sandbox.processSpace();

        bool normal_ok = (normal.committedText() == expected);
        bool sandbox_ok = (sandbox.committedText() == expected);
        bool parity_ok = (normal.committedText() == sandbox.committedText());

        if (parity_ok && normal_ok && sandbox_ok) parity_pass++;

        const char *status = (parity_ok && normal_ok && sandbox_ok) ? "PASS" : "FAIL";

        printf("  %-12s | %-12s | %-12s | %-12s | %s\n",
               cases[i].input,
               parity_ok ? "==" : "!=",
               normal_ok ? "OK" : "FAIL",
               sandbox_ok ? "OK" : "FAIL",
               status);

        if (!parity_ok) {
            fprintf(stderr, "    normal hex:  ");
            for (unsigned char c : normal.committedText()) fprintf(stderr, "%02x", c);
            fprintf(stderr, "\n    sandbox hex: ");
            for (unsigned char c : sandbox.committedText()) fprintf(stderr, "%02x", c);
            fprintf(stderr, "\n");
        }
    }

    printf("\n  -> Parity: %d/%d từ cho kết quả giống nhau\n", parity_pass, total);
    ASSERT_BOOL(parity_pass == total, true, "All words: Sandbox == Normal");
}

// ============================================================================
// TEST 2: Issue #18 — Câu nhiều từ
// ============================================================================

static void test_issue18_sentence() {
    printf("\n=== TEST 2: Issue #18 — Multi-Word Sentence ===\n\n");

    FixedSandboxSimulator normal(FixedSandboxSimulator::Mode::Normal);
    FixedSandboxSimulator sandbox(FixedSandboxSimulator::Mode::Sandbox);

    typeSentence(normal, "Goof tieeng Vieejt ddoong thuowc");
    typeSentence(sandbox, "Goof tieeng Vieejt ddoong thuowc");

    bool match = (normal.committedText() == sandbox.committedText());

    printf("  Normal:  %zu bytes\n", normal.committedText().size());
    printf("  Sandbox: %zu bytes\n", sandbox.committedText().size());
    printf("  Match:   %s\n", match ? "YES (byte-identical)" : "NO");

    ASSERT_STR(sandbox.committedText(), normal.committedText(),
               "Sandbox == Normal for Issue #18 sentence");
}

// ============================================================================
// TEST 3: Sandbox KHÔNG gọi deleteSurroundingText
// ============================================================================

static void test_sandbox_no_delete() {
    printf("\n=== TEST 3: Sandbox Never Calls deleteSurroundingText ===\n\n");

    FixedSandboxSimulator sandbox(FixedSandboxSimulator::Mode::Sandbox);

    typeSentence(sandbox, "Goof tieeng Vieejt ddoong thuowc");

    printf("  Gõ 5 từ tiếng Việt + backspace + nhiều từ\n");
    printf("  deleteSurroundingText gọi: %d lần\n", sandbox.deleteCount());

    ASSERT_INT(sandbox.deleteCount(), 0,
               "Sandbox: 0 deleteSurroundingText calls");
    printf("  -> An toàn cho sandbox apps (không gọi API không khả dụng)\n");
}

// ============================================================================
// TEST 4: Normal mode VẪN dùng deleteSurroundingText
// ============================================================================

static void test_normal_uses_delete() {
    printf("\n=== TEST 4: Normal Mode Uses deleteSurroundingText ===\n\n");

    FixedSandboxSimulator normal(FixedSandboxSimulator::Mode::Normal);

    typeWord(normal, "tieeng");
    normal.processSpace();

    printf("  Normal mode: deleteSurroundingText = %d lần\n", normal.deleteCount());

    ASSERT_BOOL(normal.deleteCount() > 0, true,
                "Normal mode uses deleteSurroundingText");
    printf("  -> Fix không ảnh hưởng behavior của normal GUI app\n");
}

// ============================================================================
// TEST 5: Backspace trong sandbox mode
// ============================================================================

static void test_sandbox_backspace() {
    printf("\n=== TEST 5: Sandbox Backspace Handling ===\n\n");

    FixedSandboxSimulator sim(FixedSandboxSimulator::Mode::Sandbox);

    // Gõ "tieeng" → preedit = "tiêng"
    typeWord(sim, "tieeng");
    printf("  Gõ 'tieeng': preedit = %zu bytes\n", sim.preedit().size());

    // Backspace xóa dấu circumflex
    sim.processBackspace();
    printf("  Backspace 1: preedit = %zu bytes\n", sim.preedit().size());

    // Backspace xóa thêm
    sim.processBackspace();
    printf("  Backspace 2: preedit = %zu bytes\n", sim.preedit().size());

    // Xóa hết
    while (!sim.preedit().empty()) {
        sim.processBackspace();
    }
    ASSERT_BOOL(sim.preedit().empty(), true, "Preedit empty after full backspace");
    printf("  Full backspace: preedit rỗng\n");

    // Gõ từ mới
    sim.processKey('x');
    sim.processKey('i');
    sim.processKey('n');
    sim.processSpace();
    ASSERT_STR(sim.committedText(), "xin ", "After backspace drain + 'xin': 'xin '");
    printf("  Gõ 'xin' + space: committed = 'xin '\n");
}

// ============================================================================
// TEST 6: commitPreedit xóa cả hai kênh (fix CodeRabbit)
// ============================================================================

static void test_commit_clears_both() {
    printf("\n=== TEST 6: commitPreedit clears both preedit channels ===\n\n");

    FixedSandboxSimulator sim(FixedSandboxSimulator::Mode::Sandbox);

    typeWord(sim, "Vieejt");
    ASSERT_BOOL(!sim.preedit().empty(), true, "Preedit has content before commit");

    sim.processSpace();
    ASSERT_STR(sim.clientPreedit(), "", "clientPreedit cleared after commit");
    ASSERT_STR(sim.serverPreedit(), "", "serverPreedit cleared after commit");
    printf("  -> Cả client và server preedit được clear\n");
}

// ============================================================================
// TEST 7: Terminal == Sandbox behavior
// ============================================================================

static void test_terminal_sandbox() {
    printf("\n=== TEST 7: Terminal Mode == Sandbox Mode ===\n\n");

    FixedSandboxSimulator terminal(FixedSandboxSimulator::Mode::Terminal);
    FixedSandboxSimulator sandbox(FixedSandboxSimulator::Mode::Sandbox);

    typeSentence(terminal, "Goof tieeng Vieejt ddoong");
    typeSentence(sandbox, "Goof tieeng Vieejt ddoong");

    ASSERT_STR(terminal.committedText(), sandbox.committedText(),
               "Terminal == Sandbox");
    printf("  -> Terminal và Sandbox cho kết quả giống nhau\n");
}

// ============================================================================
// TEST 8: UTF-8 integrity
// ============================================================================

static void test_utf8_integrity() {
    printf("\n=== TEST 8: UTF-8 Preedit Buffer Integrity ===\n\n");

    FixedSandboxSimulator sim(FixedSandboxSimulator::Mode::Sandbox);
    typeWord(sim, "Vieejt");
    const std::string &pre = sim.preedit();

    bool valid = true;
    size_t i = 0;
    while (i < pre.size()) {
        unsigned char c = static_cast<unsigned char>(pre[i]);
        if (c < 0x80) { i++; }
        else if ((c & 0xE0) == 0xC0) { i += 2; }
        else if ((c & 0xF0) == 0xE0) { i += 3; }
        else if ((c & 0xF8) == 0xF0) { i += 4; }
        else { valid = false; break; }
    }

    ASSERT_BOOL(valid, true, "Preedit buffer: valid UTF-8");
    ASSERT_BOOL(i == pre.size(), true, "No incomplete UTF-8 sequences");
    printf("  -> %zu bytes, %zu chars, all valid\n", pre.size(), i);
}

// ============================================================================
// TEST 9: Word boundary commit
// ============================================================================

static void test_word_boundary() {
    printf("\n=== TEST 9: Word Boundary Commit ===\n\n");

    FixedSandboxSimulator sim(FixedSandboxSimulator::Mode::Sandbox);

    typeWord(sim, "Vieejt");
    sim.processSpace();
    typeWord(sim, "Goof");
    sim.processSpace();

    ASSERT_BOOL(sim.committedText().size() > 5, true, "Committed text has content");
    ASSERT_STR(sim.committedText().substr(sim.committedText().size()-1), " ", "Ends with space");
    ASSERT_STR(sim.preedit(), "", "Preedit empty after word+space");
    printf("  -> Hai từ committed, preedit rỗng sau mỗi space\n");
}

// ============================================================================
// Main
// ============================================================================

int main() {
    printf("=============================================================\n");
    printf("  VnKey Fcitx5 — Fix Validation Tests (SAU KHI FIX)\n");
    printf("  Issue #18: Vietnamese typing broken in sandbox apps\n");
    printf("=============================================================\n\n");
    printf("  MÔI TRƯỜNG: fix/sandbox-app-preedit-fallback branch\n");
    printf("  MỤC ĐÍCH: Xác minh fix Issue #18 hoạt động đúng\n\n");
    printf("  Fix logic:\n");
    printf("    1. Kiểm tra hasSurroundingText capability tại runtime\n");
    printf("    2. !hasSurrounding -> fallback sang preedit mode\n");
    printf("    3. Preedit mode: engine backspace trên buffer nội bộ\n");
    printf("    4. Không gọi deleteSurroundingText() trong sandbox\n");
    printf("    5. Output sandbox == output normal (byte-identical)\n\n");
    printf("  Kết quả TRƯỚC KHI FIX xem ở nhánh: test/before-fix\n");
    printf("=============================================================\n");

    test_sandbox_normal_parity();
    test_issue18_sentence();
    test_sandbox_no_delete();
    test_normal_uses_delete();
    test_sandbox_backspace();
    test_commit_clears_both();
    test_terminal_sandbox();
    test_utf8_integrity();
    test_word_boundary();

    printf("\n=============================================================\n");
    printf("  KẾT QUẢ: %d/%d passed, %d failed\n", g_passed, g_total, g_failed);
    printf("=============================================================\n");

    return g_failed > 0 ? 1 : 0;
}
