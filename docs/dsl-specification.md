# Packet DSL 规范说明

## 概述

`.pkt` 文件是 Packet 项目的 DSL（领域特定语言），用于**定义二进制数据包的格式**。你可以把它想象成"二进制协议的 struct 定义语言"——用简洁的语法描述数据包中每个字段的名称、类型、默认值以及各种属性约束，然后由 `packet-core` 库自动完成**编码（结构体→字节）** 和**解码（字节→结构体）**。

---

## 一、文件基本结构

一个 `.pkt` 文件由三部分组成，按顺序出现：

```
[文件级属性]   ← 可选，用 #![...] 表示
[导入语句]     ← 可选，用 import("...") 表示
[结构体定义]   ← 至少一个，用 struct 关键字定义
```

示例：

```pkt
// 这是注释（支持 // 和 /* */）

#![version("1.0.0")]     // 文件级属性：版本声明
#![endian(big)]           // 文件级属性：默认字节序

import("common/types.pkt")   // 导入其他 .pkt 文件

/// 文档注释（会用 /// 自动关联到下一个结构体或导入）
#[send]
struct MyPacket {
    id: u32 = 0x12345678,
    value: u16 = 42,
}
```

---

## 二、注释

支持三种注释方式：

| 语法 | 说明 |
|------|------|
| `// 注释` | 单行注释 |
| `/* 注释 */` | 多行注释（块注释） |
| `/// 文档注释` | 文档注释，关联到紧随其后的结构体/字段/导入语句 |

---

## 三、文件级属性

文件级属性写在文件开头，使用 `#![...]` 语法。一个文件可以有零到多个文件级属性。

| 属性 | 语法 | 说明 |
|------|------|------|
| `version` | `#![version("x.y.z")]` | 声明协议版本号 |
| `endian` | `#![endian(big/little)]` | 设置文件默认字节序（默认 `big`） |
| `import_path` | `#![import_path("prefix/")]` | 设置导入文件路径前缀 |
| `doc` | `#![doc("说明")]` | 文件级文档 |
| `prefix` | `#![prefix(disable)]` | 禁用 String/Bytes/Vec\<T\> 的自动长度前缀 |

示例：

```pkt
#![version("2.0.0")]
#![endian(little)]
#![import_path("protocols/")]
#![doc("This file defines sensor protocols")]
```

**字节序说明**：

- `big`（大端序/网络字节序）：高位字节在前，如 `0x1234` 编码为 `[0x12, 0x34]`
- `little`（小端序）：低位字节在前，如 `0x1234` 编码为 `[0x34, 0x12]`
- 默认为大端序。可以通过 `#[endian(little)]` 覆盖单个字段的字节序

---

## 四、结构体定义

### 4.1 基本语法

```pkt
/// 可选文档注释
#[send]   // 或 #[receive]
struct 结构体名称 {
    字段名: 类型 = 可选默认值,
    字段名: 类型,
}
```

### 4.2 方向属性

每个结构体用 `#[send]` 或 `#[receive]` 标记方向：

| 属性 | 用途 | 要求 |
|------|------|------|
| `#[send]` | **编码**用（结构体 → 字节） | 每个字段必须有默认值 |
| `#[receive]` | **解码**用（字节 → 结构体） | 字段不需要默认值 |

> **注意**：一个结构体不能同时有 `#[send]` 和 `#[receive]`。如果都没有，则既不能编码也不能解码，仅作为被引用的子类型。

### 4.3 结构体级字节序

可以在结构体上单独指定字节序，覆盖文件级默认值：

```pkt
#[endian(little)]
struct LittleEndianPacket {
    value: u32 = 0x12345678,  // 小端序编码
}
```

### 4.4 空结构体

结构体可以没有字段：

```pkt
#[send]
struct Empty {
}
```

---

## 五、数据类型

### 5.1 基本类型

| DSL 类型 | Rust 对应类型 | 字节数 | 说明 |
|----------|-------------|--------|------|
| `u8` | `u8` | 1 | 无符号 8 位整数 |
| `u16` | `u16` | 2 | 无符号 16 位整数 |
| `u32` | `u32` | 4 | 无符号 32 位整数 |
| `u64` | `u64` | 8 | 无符号 64 位整数 |
| `u128` | `u128` | 16 | 无符号 128 位整数 |
| `i8` | `i8` | 1 | 有符号 8 位整数 |
| `i16` | `i16` | 2 | 有符号 16 位整数 |
| `i32` | `i32` | 4 | 有符号 32 位整数 |
| `i64` | `i64` | 8 | 有符号 64 位整数 |
| `i128` | `i128` | 16 | 有符号 128 位整数 |
| `f32` | `f32` | 4 | IEEE 754 单精度浮点数 |
| `f64` | `f64` | 8 | IEEE 754 双精度浮点数 |
| `bool` | `bool` | 1 | 布尔值（`true` = 0x01, `false` = 0x00） |

### 5.2 复合类型

| DSL 类型 | 说明 | 编码格式 |
|----------|------|----------|
| `String` | UTF-8 字符串 | 4 字节长度前缀（大端 u32）+ UTF-8 内容 |
| `Bytes` | 原始字节数组 | 4 字节长度前缀（大端 u32）+ 字节内容 |
| `Vec<T>` | 变长数组（元素类型为 T） | 4 字节长度前缀（大端 u32）+ 连续元素 |
| `[T; N]` | 定长数组（N 个元素，类型为 T） | 连续 N 个元素，无长度前缀 |
| `TypeName` | 自定义结构体类型 | 按被引用结构体的字段定义递归编码 |

示例：

```pkt
struct ComplexTypes {
    str_val: String = "hello",
    bytes_val: Bytes = 0xDEADBEEF,
    vec_val: Vec<u32> = [1, 2, 3],
    arr_val: [u8; 4] = [0x01, 0x02, 0x03, 0x04],
    nested: Vec<Vec<u8>> = [[1, 2], [3, 4]],
    custom: MyType = MyType { field1: 1, field2: 2 },
}
```

**编码布局详解**：

| 类型 | 二进制布局 |
|------|-----------|
| `String` | `[长度(u32)] [UTF-8字节...]` |
| `Bytes` | `[长度(u32)] [原始字节...]` |
| `Vec<u32>` | `[元素个数(u32)] [元素1] [元素2] ...` |
| `[u8; 4]` | `[字节1] [字节2] [字节3] [字节4]` |

> **关键理解**：`String`、`Bytes`、`Vec<T>` 的 4 字节长度前缀是 **codec 内部自动管理的**——编码时自动写入，解码时自动读取。你在 `.pkt` 定义文件中**不需要**额外写一个 `length: u32` 字段来记录它们的长度。举例：
>
> ```pkt
> #[send]
> struct Message {
>     payload: Vec<u8> = [10, 20, 30],  // ← 只管数据本身
> }
> // 编码结果自动为: [0x00, 0x00, 0x00, 0x03] [0x0A, 0x0B, 0x1E]
> //  └──── 长度前缀(codec自动填) ────┘  └─── 数据 ───┘
> ```
>
> 只有一种情况你需要手动处理长度：**协议设计本身就要求在报文头部放一个总长度字段**（如很多网络协议），此时用 `#[auto]` 属性实现，详见 7.2 节。
>
> 另：`String`、`Bytes`、`Vec<T>` 的长度前缀始终使用**大端序**（不受 `#[endian]` 影响），只有元素值受字节序设置影响。
>
> **禁用自动前缀**：如果协议设计希望自己管理长度（例如使用 `#[auto]` + `#[len_ref]` 定义自定义长度字段），可以通过全局属性 `#![prefix(disable)]` 禁用自动前缀，详见 7.10 节。

### 5.3 类型大小速查

| 类型 | 大小 |
|------|------|
| `u8` / `i8` / `bool` | 1 字节 |
| `u16` / `i16` | 2 字节 |
| `u32` / `i32` / `f32` | 4 字节 |
| `u64` / `i64` / `f64` | 8 字节 |
| `u128` / `i128` | 16 字节 |
| `[T; N]` | N × sizeof(T) 字节 |
| `String` / `Bytes` / `Vec<T>` | 变长（动态大小） |

---

## 六、字段定义

### 6.1 基本语法

```pkt
字段名: 类型 = 可选默认值,
```

- 字段名必须以字母或下划线开头，后可跟字母、数字、下划线
- 类型必须是上述支持的类型之一
- 默认值在 `#[send]` 结构体中**必须提供**（除非有 `#[auto]` 属性）
- 默认值在 `#[receive]` 结构体中**不能提供**

### 6.2 默认值格式

| 类型 | 示例 |
|------|------|
| 整数 | `42`, `-10`, `0xFF`, `0x12345678` |
| 浮点数 | `3.14`, `-2.5` |
| 布尔 | `true`, `false` |
| 字符串 | `"hello"` |
| 列表 | `[1, 2, 3]` |
| 结构体初始化 | `TypeName { field1: val1, field2: val2 }` |

---

## 七、字段属性

字段属性用 `#[...]` 写在字段名前，是 DSL 最强大的部分。

### 7.1 属性总览

| 属性 | 语法 | 适用字段类型 | 说明 |
|------|------|-------------|------|
| `auto` | `#[auto]` | 无符号整数 | 自动计算整个数据包的总字节数 |
| `auto` | `#[auto(field)]` | 无符号整数 | 自动计算指定字段的元素个数 |
| `len_ref` | `#[len_ref(field)]` | 无符号整数 | 引用字段长度（解码 Vec/String/Bytes 时用） |
| `remaining` | `#[remaining]` | Bytes/String/Vec\<T\> | 捕获数据包剩余所有字节 |
| `if` | `#[if(condition)]` | 任意类型 | 条件字段，依赖前面的 bool 字段 |
| `bits` | `#[bits(start, end)]` | u8/u16/u32/u64 | 位域（多比特范围） |
| `bit` | `#[bit(pos)]` | bool 或整数 | 位域（单比特） |
| `endian` | `#[endian(big/little)]` | 数值类型 | 覆盖该字段的字节序 |
| `checksum` | `#[checksum(algo)]` | 无符号整数 | 计算前面所有字段的校验和 |

### 7.2 auto — 自动长度

用在不带默认值的字段上，在编码时自动回填长度值。

**`#[auto]` — 总长度**

自动计算并填入整个数据包的总字节数：

```pkt
#[send]
struct Packet {
    #[auto]           // 编码时自动写入整个数据包的总字节数
    total_len: u32,   // 注意：这里没有 = 默认值
    header: u32 = 0x1234,
    payload: [u8; 4] = [1, 2, 3, 4],
}
// 编码结果: [0x00, 0x00, 0x00, 0x0C] ← 总长12字节
//           [0x12, 0x34, 0x00, 0x00]
//           [0x01, 0x02, 0x03, 0x04]
```

**`#[auto(field_name)]` — 字段元素个数**

自动计算并填入指定 Vec/Bytes/String 字段的**元素个数**：

```pkt
#[send]
struct VariablePacket {
    #[auto(items)]    // 编码时自动填入 items 的元素个数
    count: u8,        // 注意：没有 = 默认值
    items: Vec<u32> = [100, 200, 300],
}
// 编码结果: [0x03] ← items 有 3 个元素
//           [0x00, 0x00, 0x00, 0x03] ← Vec 长度前缀
//           [0x00, 0x00, 0x00, 0x64] ← 100
//           [0x00, 0x00, 0x00, 0xC8] ← 200
//           [0x00, 0x00, 0x01, 0x2C] ← 300
```

### 7.3 len_ref — 长度引用

仅用于 `#[receive]` 结构体。告诉解码器：这个 Vec/String/Bytes 字段的长度由前面某个字段的值决定。

```pkt
#[receive]
struct VariableData {
    count: u8,               // 先解码出 count = 3
    #[len_ref(count)]        // 告诉解码器：items 有 count 个元素
    items: Vec<u32>,         // 于是解码 3 个 u32
}
```

对比 `#[auto]` 和 `#[len_ref]`：

| 属性 | 用途 | 用在哪个方向 |
|------|------|-------------|
| `#[auto]` | 编码时自动填入长度值 | `#[send]`（编码时计算） |
| `#[len_ref]` | 解码时告诉如何读取变长数据 | `#[receive]`（解码时使用） |

### 7.4 remaining — 剩余字节

捕获从当前位置到数据结尾的所有剩余字节。

> **与普通 `Bytes`/`Vec<T>` 的区别**：普通 `String`/`Bytes`/`Vec<T>` 带有 4 字节自动长度前缀（见 5.2）；而 `#[remaining]` **不读也不写长度前缀**，直接消费/写入数据包的全部剩余字节。

**约束**：
- 只能用于 `#[receive]` 结构体
- 必须是结构体的**最后一个字段**
- 只能用在 `Bytes`、`String` 或 `Vec<u8>` 类型上
- 一个结构体最多只能有一个 `#[remaining]` 字段

```pkt
#[receive]
struct Message {
    header: u32,           // 固定解码 4 字节
    #[remaining]           // 剩下的所有字节都归 payload
    payload: Bytes,
}
// 如果收到的数据是 [0x12, 0x34, 0x56, 0x78, 0x01, 0x02, 0x03]
// 则 header = 0x12345678, payload = [0x01, 0x02, 0x03]
```

```pkt
#[receive]
struct VariablePayload {
    #[remaining]
    data: Vec<u8>,        // 剩余字节逐个解码为 Vec<u8>
}
```

### 7.5 if — 条件字段

根据前面的 `bool` 字段决定是否编码/解码当前字段。

```pkt
#[send]
struct ConditionalPacket {
    has_data: bool = true,
    #[if(has_data)]        // 只有 has_data = true 时才编码 data
    data: u32 = 100,
}
// has_data = true  → 编码 5 字节: [0x01, 0x00, 0x00, 0x00, 0x64]
// has_data = false → 编码 1 字节: [0x00]
```

```pkt
#[receive]
struct ConditionalRecv {
    has_data: bool,
    #[if(has_data)]
    data: u32,            // 只有 has_data = true 时才解码 data
}
```

### 7.6 bits / bit — 位域

将多个小字段打包到同一个字节（或多个字节）中，常用于状态寄存器等场景。

- `#[bits(start, end)]`：提取第 start 位到第 end 位的连续比特范围
- `#[bit(pos)]`：提取第 pos 位的单个比特

多个位域字段会根据它们占用的最大位位置，自动分组到合适的存储单元（8/16/32/64 位）。

```pkt
#[send]
struct StatusRegister {
    #[bits(0, 3)]    // 位 0~3：值为 5
    code: u8 = 5,
    #[bit(4)]        // 位 4：值为 true
    active: bool = true,
    #[bits(5, 7)]    // 位 5~7：值为 2
    mode: u8 = 2,
}
// 编码结果（1 字节）:
// 位 7 6 5 | 4 | 3 2 1 0
//   0 1 0 | 1 | 0 1 0 1
// 二进制: 0101 0101 = 0x55
```

```pkt
#[receive]
struct StatusRecv {
    #[bits(0, 3)]
    code: u8,
    #[bit(4)]
    active: bool,
    #[bits(5, 7)]
    mode: u8,
}
```

> 解码后的位域值类型：`#[bits]` 字段解码为 `u64`、`#[bit]` 字段解码为 `bool`。

### 7.7 endian — 字段级字节序

覆盖文件级或结构体级的默认字节序，仅对该字段生效。

```pkt
#[send]
struct MixedEndianPacket {
    field_a: u32 = 0x12345678,
    #[endian(little)]
    field_b: u32 = 0x12345678,
}
// 编码结果:
// field_a (大端序 - 文件默认): [0x12, 0x34, 0x56, 0x78]
// field_b (小端序 - 字段覆盖): [0x78, 0x56, 0x34, 0x12]
```

### 7.8 checksum — 校验和

自动计算前面所有字段的校验和，填入该字段的占位位置。

**支持的算法**：

| 算法 | 关键字 | 说明 |
|------|--------|------|
| CRC-8 | `crc8` | 8 位 CRC |
| CRC-16 | `crc16` | Modbus CRC-16 |
| CRC-32 | `crc32` | IEEE 802.3 CRC-32 |
| XOR | `xor` | 逐字节异或 |
| Sum | `sum` | 字节累加和 |

```pkt
#[send]
struct PacketWithChecksum {
    data: u32 = 0x12345678,
    #[checksum(crc32)]    // 计算 data 字段的 CRC32
    crc: u32 = 0,         // 编码时自动填入计算结果
}

// 使用 XOR 校验：
#[send]
struct XorPacket {
    data: u32 = 0x12345678,
    #[checksum(xor)]
    checksum: u8 = 0,     // 编码后: 0x08
}
```

> 校验和字段必须是无符号整数类型（`u8`/`u16`/`u32`/`u64`）。

### 7.9 多个属性组合

一个字段可以有多个属性，顺序任意：

```pkt
#[send]
struct ComplexPacket {
    has_data: bool = true,
    count: u8 = 3,
    #[if(has_data)]          // 条件：has_data 为 true 时才有
    #[auto(count)]           // 自动计算 items 的元素个数
    data: Vec<u8> = [1, 2, 3],
}
```

### 7.10 prefix(disable) — 禁用自动长度前缀

默认情况下，`String`、`Bytes`、`Vec<T>` 在编解码时自动带有 4 字节长度前缀（见 5.2）。使用文件级属性 `#![prefix(disable)]` 可以禁用此行为，让用户通过 `#[auto]` 和 `#[len_ref]` 自行管理长度。

**编码行为变化**：

| 类型 | 默认（有前缀） | `#![prefix(disable)]` 后 |
|------|---------------|--------------------------|
| `String` | `[4字节长度] [UTF-8数据]` | 只有 `[UTF-8数据]` |
| `Bytes` | `[4字节长度] [原始字节]` | 只有 `[原始字节]` |
| `Vec<T>` | `[4字节元素数] [元素...]` | 只有 `[元素...]` |

**要求**：当 `#![prefix(disable)]` 生效时，`#[receive]` 结构体中的 `String`/`Bytes`/`Vec<T>` 字段**必须**使用 `#[len_ref]` 或 `#[remaining]` 指明长度，否则校验器报错 **E016**。

**示例**：

```pkt
#![prefix(disable)]

#[send]
struct ManualLenPacket {
    #[auto(data)]
    count: u8,
    data: Vec<u8> = [10, 20, 30],
}

#[receive]
struct ManualLenRecv {
    count: u8,
    #[len_ref(count)]
    data: Vec<u8>,
}
// 编码结果（无 4 字节前缀）:
// [0x03] ← count = 3（由 #[auto] 自动计算）
// [0x0A, 0x0B, 0x1E] ← data 本体
```

**Bytes 使用十六进制值**：

```pkt
#![prefix(disable)]

#[send]
struct BytesPacket {
    #[auto(payload)]
    len: u8,
    payload: Bytes = 0xBEAD,   // 编码为 2 字节: [0xBE, 0xAD]
}

#[receive]
struct BytesRecv {
    len: u8,
    #[len_ref(len)]
    payload: Bytes,
}
// 编码结果（共 3 字节）:
// [0x02] ← payload 长度为 2 字节
// [0xBE, 0xAD] ← payload 数据
```

> **注意**：`#![prefix(disable)]` 不影响定长类型，也不影响 `#[auto]`、`#[len_ref]`、`#[remaining]` 等其他属性的行为。（`remaining` 本身就不读写长度前缀，因此不受影响。）

---

## 八、导入系统

### 8.1 导入语句

使用 `import("文件路径")` 导入其他 `.pkt` 文件中定义的结构体：

```pkt
import("common/types.pkt")
import("network/protocols.pkt")
```

### 8.2 导入路径解析

- 导入路径是相对于当前文件所在目录的**相对路径**
- 可以用 `#![import_path("前缀/")]` 设置全局导入路径前缀
- 支持递归导入，但会检测**循环导入**并报错

### 8.3 导入与文档

可以为导入语句添加文档注释：

```pkt
/// Common types for sensors
import("common/sensor.pkt")
```

### 8.4 合并规则

导入的结构体被合并到当前 schema：
- 结构体名称在合并后的整体中必须**唯一**
- 合并后 `file_attribute` 也会合并（不会重复添加相同的属性）

---

## 九、嵌套结构体

结构体可以作为另一个结构体的字段类型使用，支持任意层次的嵌套。

```pkt
// 定义一个子结构体（没有方向，仅作为类型被引用）
struct Address {
    street: String = "123 Main St",
    city: String = "Springfield",
}

// 在另一个结构体中引用
#[send]
struct Person {
    name: String = "Alice",
    age: u8 = 30,
    address: Address = Address { street: "123 Main St", city: "Springfield" },
}

#[receive]
struct PersonRecv {
    name: String,
    age: u8,
    address: Address,
}
```

嵌套结构体的编码是递归展开的——先编码 `name`，再编码 `age`，再编码 `Address` 的每个字段。

---

## 十、二进制编码格式速查

### 10.1 基本布局

```
┌─────────────────────────────────────────┐
│  字段 1：固定长度                        │
├─────────────────────────────────────────┤
│  字段 2：可变长度 (String/Bytes/Vec)    │
│  ┌──────────┬────────────────────────┐  │
│  │ 长度(u32)│        内容           │  │
│  │ 大端序   │   (受字节序影响)       │  │
│  └──────────┴────────────────────────┘  │
├─────────────────────────────────────────┤
│  字段 3：定长数组 [T; N]                │
│  ┌──────┬──────┬──────┬──────┐         │
│  │ [0]  │ [1]  │ ...  │ [N-1]│         │
│  └──────┴──────┴──────┴──────┘         │
├─────────────────────────────────────────┤
│  字段 4：位域 (打包到 1/2/4/8 字节)     │
│  ┌─┬─┬─┬─┬─┬─┬─┬─┐                    │
│  │7│6│5│4│3│2│1│0│ ← 位索引           │
│  └─┴─┴─┴─┴─┴─┴─┴─┘                    │
└─────────────────────────────────────────┘
```

### 10.2 各类型编码长度

| 类型 | 编码占用字节 |
|------|------------|
| `u8` / `i8` / `bool` | 1 |
| `u16` / `i16` | 2 |
| `u32` / `i32` / `f32` | 4 |
| `u64` / `i64` / `f64` | 8 |
| `u128` / `i128` | 16 |
| `String` | 4 + len(UTF-8 bytes) |
| `Bytes` | 4 + len(raw bytes) |
| `Vec<T>` | 4 + N × sizeof(T) |
| `[T; N]` | N × sizeof(T)（无前缀） |

> **重要**：`String`、`Bytes`、`Vec<T>` 的 4 字节长度前缀由 codec **自动管理**——定义文件里不需要手动写一个长度字段。这些长度前缀始终使用**大端序**编码（不受 `#[endian]` 影响）。可通过 `#![prefix(disable)]` 禁用此行为（详见 7.10）。

### 10.3 字节序作用范围

```
文件级 #![endian(X)]           ← 默认值，影响所有字段
  └── 结构体 #[endian(Y)]      ← 覆盖此结构体内的所有字段
       └── 字段 #[endian(Z)]   ← 仅覆盖该字段
```

`String`/`Bytes`/`Vec<T>` 的**长度前缀**不受字节序影响，始终为大端序。只有**元素值**受字节序影响。

---

## 十一、校验错误码速查

在调用 `validate_schema()` 验证 schema 时，可能产生以下错误：

| 代码 | 说明 |
|------|------|
| **E001** | 引用的结构体不存在 |
| **E002** | 结构体名称重复 |
| **E003** | 字段名称重复（同一结构体内） |
| **E004** | send 结构体的字段缺少默认值 |
| **E005** | 字段名为空 |
| **E006** | `#[if()]` 引用了不存在的字段或非 bool 字段 |
| **E007** | `#[len_ref()]` 引用了不存在的字段或非整数字段 |
| **E008** | `#[checksum]` 字段不是无符号整数类型 |
| **E009** | 引用了未定义的类型（找不到对应的自定义结构体） |
| **E010** | 数组长度为零（`[T; 0]` 不允许） |
| **E011** | `#[remaining]` 字段不是 Bytes/String/Vec\<T\> 类型 |
| **E012** | `#[remaining]` 出现在非 receive 结构体中 |
| **E013** | `#[remaining]` 不是最后一个字段 |
| **E014** | 多个 `#[remaining]` 字段 |
| **E015** | `#[auto(field)]` 引用了不存在的字段 |
| **E016** | `#![prefix(disable)]` 模式下，receive 结构体的 String/Bytes/Vec\<T\> 字段缺少 `#[len_ref]` 或 `#[remaining]` |

---

## 十二、完整示例

### 12.1 传感器数据协议

```pkt
// sensor.pkt
#![version("1.0.0")]
#![endian(big)]

/// Sample structure within packet
struct Sample {
    /// Timestamp in milliseconds
    timestamp: u32,
    /// Temperature in Celsius
    temperature: f32,
    /// Humidity percentage
    humidity: u8,
}

/// Sensor data packet (send direction)
#[send]
struct SensorData {
    /// Device identifier (0x12345678)
    device_id: u32 = 0x12345678,
    /// Number of samples
    count: u8 = 2,
    /// Sample data
    samples: Vec<Sample> = [
        Sample { timestamp: 1000, temperature: 25.5, humidity: 60 },
        Sample { timestamp: 2000, temperature: 26.0, humidity: 62 },
    ],
    /// CRC32 checksum of entire packet
    #[checksum(crc32)]
    crc: u32 = 0,
}

/// Sensor data packet (receive direction)
#[receive]
struct SensorDataRecv {
    device_id: u32,
    count: u8,
    samples: Vec<Sample>,
    crc: u32,
}
```

### 12.2 网络协议头

```pkt
// network.pkt
#![endian(big)]

/// Network packet header
#[send]
struct NetHeader {
    /// Protocol version
    version: u8 = 1,
    /// Packet type (4 bits) and flags (4 bits)
    #[bits(0, 3)]
    pkt_type: u8 = 0,
    #[bits(4, 7)]
    flags: u8 = 0,
    /// Payload length (auto-calculated)
    #[auto]
    length: u16,
    /// Payload data
    payload: Vec<u8> = [0x01, 0x02, 0x03],
}

/// Network packet header (receive)
#[receive]
struct NetHeaderRecv {
    version: u8,
    #[bits(0, 3)]
    pkt_type: u8,
    #[bits(4, 7)]
    flags: u8,
    length: u16,
    #[len_ref(length)]
    payload: Vec<u8>,
}
```

### 12.3 结构体初始化语法

在给自定义结构体类型字段提供默认值时，使用结构体初始化语法：

```pkt
struct Point {
    x: i32 = 0,
    y: i32 = 0,
}

#[send]
struct Line {
    start: Point = Point { x: 0, y: 0 },
    end: Point = Point { x: 100, y: 200 },
}
```

---

## 十三、快速上手

### 13.1 在 Rust 中使用

```rust
use packet_core::{parse_schema, validate_schema, Codec};

// 1. 定义 DSL
let source = r#"
    #[send]
    struct MyPacket {
        id: u32 = 0x12345678,
        value: u16 = 42,
    }

    #[receive]
    struct MyPacketRecv {
        id: u32,
        value: u16,
    }
"#;

// 2. 解析
let schema = parse_schema(source)?;

// 3. 验证
validate_schema(&schema)?;

// 4. 编码
let send_codec = Codec::compile(&schema, "MyPacket")?;
let bytes = send_codec.encode()?;
// bytes = [0x12, 0x34, 0x56, 0x78, 0x00, 0x2A]

// 5. 解码
let recv_codec = Codec::compile(&schema, "MyPacketRecv")?;
let decoded = recv_codec.decode(&bytes)?;
// decoded.fields[0] = ("id", U32(0x12345678))
// decoded.fields[1] = ("value", U16(42))
```

### 13.2 从文件加载

```rust
use packet_core::loader::load_schema;

let schema = load_schema(std::path::Path::new("my_protocol.pkt"))?;
```

---

## 十四、与 Protocol Buffers 的对比

| 特性 | Packet DSL | Protocol Buffers |
|------|-----------|-----------------|
| 适用范围 | 二进制网络协议、文件格式 | 通用序列化 |
| 字节序控制 | 支持（字段级） | 固定小端序 |
| 位域操作 | 原生支持 | 不支持 |
| 条件字段 | 支持 `#[if]` | 不支持 |
| 自动长度 | 支持 `#[auto]` | 部分支持 |
| 校验和 | 内置多种算法 | 不支持 |
| 版本管理 | 文件级 `#![version]` | `package` + 字段编号 |
| 字段编号 | 按顺序，无需编号 | 每个字段需编号 |

---

> **提示**：`.pkt` 文件的扩展名可以是 `.pkt`（默认）或 `.packet`（替代）。
