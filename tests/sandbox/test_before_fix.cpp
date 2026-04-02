/*
 * test_before_fix.cpp — Cấp độ 2+ Test: Chứng minh bug Issue #18 tồn tại
 *
 * MÔI TRƯỜNG: master branch (TRƯỚC khi apply fix)
 *
 * Test này chứng minh rằng code hiện tại (master) bị lỗi khi gõ tiếng Việt
 * trong sandbox app (Flatpak/Snap/Terminal):
 *   - Sandbox app không hỗ SurroundingText capability
 *   - Code hiện tại luôn dùng direct commit mode (không fallback)
 *   - deleteSurroundingText() bị silent fail → ký tự cũ không bị xóa
 *   - Kết quả: ký tự bị nhân đôi (vd: "tiêng" → "tieengếeng")
 *
 * Xem kết quả tương phản ở nhánh test/after-fix.
 *
 * Build:
 *   g++ -std=c++17 -o test_before_fix test_before_fix.cpp \
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
// Mock InputContext — tái hiện hành vi Fcitx5 sandbox app
// ============================================================================
//
// Trong sandbox app (Flatpak/Snap), hasSurroundingText = false.
// Khi deleteSurroundingText() được gọi → silent fail (không làm gì).
// Đây là ROOT CAUSE của Issue #18.
// ============================================================================

struct MockInputContext {
    bool hasSurroundingText = true;  // true = GUI, false = sandbox
    std::string committedText;
    std::string clientPreedit;
    std::string serverPreedit;
    int deleteSurroundingTextCount = 0;

    void commitString(const std::string &s) { committedText += s; }

    void deleteSurroundingText(int charOffset, unsigned int charLen) {
        deleteSurroundingTextCount++;
        if (!hasSurroundingText) {
            // SANDBOX BUG: silent fail — không xóa ký tự cũ!
            return;
        }
        // Normal app: xóa ký tự theo character offset (FCcitx5 API)
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
// SandboxSimulator — Mô phỏng BEHAVIOR CỦA CODE HIỆN TẠI (master)
//
// Đây là logic TRONG vnkey-fcitx5.cpp TRÊN MASTER BRANCH:
//   - KHÔNG kiểm tra hasSurroundingText capability
//   - LUÔN dùng direct commit mode
//   - Gọi deleteSurroundingText() → bị silent fail trong sandbox
//   → Ký tự bị nhân đôi
// ============================================================================

class SandboxSimulator {
public:
    SandboxSimulator() : engine_(nullptr) {
        engine_ = vnkey_engine_new();
        vnkey_engine_set_input_method(engine_, 0); // Telex
        vnkey_engine_set_viet_mode(engine_, 1);
        vnkey_engine_set_options(engine_, 1, 1, 1, 1, 0, 0);
        // Sandbox app: không có SurroundingText capability
        ic_.hasSurroundingText = false;
    }

    ~SandboxSimulator() {
        if (engine_) vnkey_engine_free(engine_);
    }

    void reset() {
        vnkey_engine_reset(engine_);
        ic_.committedText.clear();
        ic_.deleteSurroundingTextCount = 0;
    }

    // Gõ một phím — tái hiện logic CŨ trong VnKeyState::keyEvent()
    // Code cũ: LUÔN dùng direct commit, không phân biệt sandbox/normal
    void processKey(char c) {
        uint32_t keyCode = static_cast<uint32_t>(static_cast<unsigned char>(c));
        uint8_t buf[256];
        size_t actualLen = 0;
        size_t backspaces = 0;

        int processed = vnkey_engine_process(
            engine_, keyCode, buf, sizeof(buf), &actualLen, &backspaces, nullptr);

        // LOGIC CŨ: luôn direct commit
        if (processed) {
            if (backspaces > 0) {
                // BUG: gọi deleteSurroundingText nhưng sandbox app bỏ qua!
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

    void processSpace() {
        vnkey_engine_soft_reset(engine_);
        ic_.commitString(" ");
    }

    const std::string& committedText() const { return ic_.committedText; }
    int deleteCount() const { return ic_.deleteSurroundingTextCount; }

private:
    ::VnKeyEngine *engine_;
    MockInputContext ic_;
};

// ============================================================================
// Helper: gõ một từ/câu
// ============================================================================

static void typeWord(SandboxSimulator &sim, const char *word) {
    for (const char *p = word; *p; p++) {
        sim.processKey(*p);
    }
}

static void typeSentence(SandboxSimulator &sim, const char *sentence) {
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
// TEST 1: Chứng minh bug — Output sandbox KHÔNG khớp output đúng
//
// Mỗi từ Telex sẽ:
//   - Engine yêu cầu backspace (xóa ký tự cũ)
//   - Code cũ gọi deleteSurroundingText() → silent fail
//   - Ký tự mới được commit nhưng ký tự cũ vẫn còn → nhân đôi
// ============================================================================

static void test_bug_doubled_characters() {
    printf("\n=== TEST 1: Bug — Ký tự bị nhân đôi trong sandbox app ===\n");
    printf("  (Đây là behavior của code HIỆN TẠI trên master)\n\n");

    struct TestCase {
        const char *input;       // Telex input
        const char *correct_hex; // Output đúng (hex UTF-8)
        const char *description;
    };

    TestCase cases[] = {
        {"tieeng",  "7469c3aa6e67",     "ee -> circumflex: 'tieng' -> 'tieng'"},
        {"Goof",    "47e1bb93",         "oo->circumflex, f->grave: 'Goof' -> 'Go'"},
        {"Vieejt",  "5669e1bb8774",     "ee->circumflex, j->dot-below: 'Viet'"},
        {"ddoong",  "c491c3b46e67",     "dd->d-bar, oo->circumflex: 'dong'"},
        {"thuowc",  "7468c6b0c6a163",   "ow->horn: 'thuoc'"},
        {nullptr, nullptr, nullptr}
    };

    int bug_confirmed = 0;
    int total = 0;

    printf("  %-12s | %-12s | %-20s | %-10s\n",
           "Telex Input", "Output đúng", "Output sandbox (broken)", "Kết quả");
    printf("  %-12s-|-%-12s-|-%-20s-|-%-10s\n",
           "------------", "------------", "--------------------", "----------");

    for (int i = 0; cases[i].input; i++) {
        total++;
        std::string correct = hexToStr(cases[i].correct_hex) + " ";

        SandboxSimulator sim;
        typeWord(sim, cases[i].input);
        sim.processSpace();

        bool is_wrong = (sim.committedText() != correct);

        if (is_wrong) bug_confirmed++;

        const char *status = is_wrong ? "SAI" : "dung (?)";

        printf("  %-12s | ", cases[i].input);
        // Print correct output as UTF-8
        for (unsigned char c : correct) {
            if (c < 0x80) printf("%c", c);
            else printf("?");
        }
        printf("      | ");
        // Print actual output
        for (unsigned char c : sim.committedText()) {
            if (c < 0x80) printf("%c", c);
            else printf("?");
        }
        printf("%-13s| %-10s\n", "", status);

        if (!is_wrong) {
            fprintf(stderr, "    WARNING: expected wrong output but got correct!\n");
            fprintf(stderr, "    output hex:   ");
            for (unsigned char c : sim.committedText()) fprintf(stderr, "%02x", c);
            fprintf(stderr, "\n    correct hex: ");
            for (unsigned char c : correct) fprintf(stderr, "%02x", c);
            fprintf(stderr, "\n");
        } else {
            // Show the broken output hex for debugging
            fprintf(stderr, "    broken hex:   ");
            for (unsigned char c : sim.committedText()) fprintf(stderr, "%02x", c);
            fprintf(stderr, "\n    correct hex:  ");
            for (unsigned char c : correct) fprintf(stderr, "%02x", c);
            fprintf(stderr, "\n");
        }
    }

    printf("\n");
    printf("  -> BUG XÁC NHẬN: %d/%d từ bị sai output\n", bug_confirmed, total);

    // Test passes when ALL outputs are wrong (proving the bug exists)
    ASSERT_BOOL(bug_confirmed == total, true,
                "All words produce wrong output in sandbox (bug confirmed)");
}

// ============================================================================
// TEST 2: Bug trên câu nhiều từ (Issue #18 exact scenario)
// ============================================================================

static void test_bug_sentence() {
    printf("\n=== TEST 2: Bug trên câu nhiều từ (Issue #18) ===\n");
    printf("  Gõ: 'Goof tieeng Vieejt'\n\n");

    SandboxSimulator sim;
    typeSentence(sim, "Goof tieeng Vieejt");

    std::string correct = hexToStr("47e1bb93") + " " + hexToStr("7469c3aa6e67") + " " + hexToStr("5669e1bb8774") + " ";

    bool is_wrong = (sim.committedText() != correct);

    printf("  Output đúng:   ");
    for (unsigned char c : correct) {
        if (c < 0x80) printf("%c", c);
        else printf("?");
    }
    printf("\n");

    printf("  Output sandbox: ");
    for (unsigned char c : sim.committedText()) {
        if (c < 0x80) printf("%c", c);
        else printf("?");
    }
    printf("\n");

    printf("  Output hex:    ");
    for (unsigned char c : sim.committedText()) fprintf(stderr, "%02x", c);
    printf("\n");

    printf("  Correct hex:   ");
    for (unsigned char c : correct) fprintf(stderr, "%02x", c);
    printf("\n");

    ASSERT_BOOL(is_wrong, true,
                "Sandbox sentence produces wrong output (bug confirmed)");

    // deleteSurroundingText được gọi nhưng không làm gì (silent fail)
    printf("  deleteSurroundingText called: %d times (tất cả bị bỏ qua)\n",
           sim.deleteCount());
    ASSERT_BOOL(sim.deleteCount() > 0, true,
                "deleteSurroundingText was called but had no effect (root cause)");
}

// ============================================================================
// TEST 3: Root cause — deleteSurroundingText bị bỏ qua trong sandbox
// ============================================================================

static void test_root_cause() {
    printf("\n=== TEST 3: Root Cause Analysis ===\n\n");

    SandboxSimulator sim;

    // Gõ 'tieeng' — engine sẽ yêu cầu xóa 'e' trước khi thêm 'ê'
    typeWord(sim, "tieeng");
    sim.processSpace();

    printf("  Input: 'tieeng' + space\n");
    printf("  Output đúng:    'tieng ' (e -> ê)\n");
    printf("  Output sandbox: ");
    for (unsigned char c : sim.committedText()) {
        if (c < 0x80) printf("%c", c);
        else printf("?");
    }
    printf("\n");
    printf("  deleteSurroundingText gọi: %d lần\n", sim.deleteCount());
    printf("  deleteSurroundingText hiệu quả: 0 lần (silent fail!)\n");

    // Ghi chú root cause
    printf("\n  ROOT CAUSE:\n");
    printf("    1. Engine yêu cầu backspace: 'tieeng' -> cần xóa 'e' trước\n");
    printf("    2. Code gọi deleteSurroundingText(-1, 1)\n");
    printf("    3. Sandbox app KHÔNG hỗ trợ SurroundingText capability\n");
    printf("    4. Fcitx5 API: deleteSurroundingText() bị bỏ qua (no-op)\n");
    printf("    5. Ký tự 'ê' được commit NHƯNG 'e' cũ vẫn còn\n");
    printf("    6. Kết quả: 'tieeng' thay vì 'tieng'\n");

    ASSERT_BOOL(sim.deleteCount() > 0, true,
                "deleteSurroundingText was called (proving code tried to delete)");
    ASSERT_BOOL(sim.committedText() != hexToStr("7469c3aa6e67") + " ", true,
                "But output is still wrong (proving delete had no effect)");
}

// ============================================================================
// Main
// ============================================================================

int main() {
    printf("=============================================================\n");
    printf("  VnKey Fcitx5 — Bug Reproduction Tests (TRƯỚC KHI FIX)\n");
    printf("  Issue #18: Vietnamese typing broken in sandbox apps\n");
    printf("=============================================================\n\n");
    printf("  MÔI TRƯỜNG: master branch (chưa có fix)\n");
    printf("  MỤC ĐÍCH: Chứng minh bug tồn tại trong code hiện tại\n\n");
    printf("  Cách hoạt động:\n");
    printf("    - SandboxSimulator mô phỏng chính xác logic code CŨ\n");
    printf("    - MockInputContext: hasSurroundingText = false (sandbox)\n");
    printf("    - deleteSurroundingText() bị silent fail (như Fcitx5 thực tế)\n");
    printf("    - Kết quả: ký tự bị nhân đôi\n\n");
    printf("  Kết quả tương phản xem ở nhánh: test/after-fix\n");
    printf("=============================================================\n");

    test_bug_doubled_characters();
    test_bug_sentence();
    test_root_cause();

    printf("\n=============================================================\n");
    printf("  KẾT QUẢ: %d/%d passed, %d failed\n", g_passed, g_total, g_failed);
    printf("=============================================================\n");
    printf("\n  %s\n",
        g_failed == 0
        ? ">> TẤT CẢ TEST PASS >> Bug đã được xác nhận tồn tại trên master <<"
        : ">> CÓ TEST FAIL >> Kiểm tra lại logic simulator <<");
    printf("  >> Xem kết quả SAU KHI FIX ở nhánh: test/after-fix <<\n");

    return g_failed > 0 ? 1 : 0;
}
