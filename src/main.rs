use std::collections::HashMap;
use std::env;
use std::fs::File;
use std::io::{prelude::*, stdin, BufReader};
use std::string::String;

// 仅数字
fn digital(x: u8, _y: u8) -> bool {
    return x >= 48 && x <= 57;
}

// 包含数字和.号
fn digital_dot(x: u8, _y: u8) -> bool {
    return (x >= 48 && x <= 57) || x == 46;
}

// 包含数字字母和.号或:号（IPv4或IPv6）
fn digital_dot_colon(x: u8, _y: u8) -> bool {
    return (x >= 48 && x <= 58) || x == 46 || (x >= 97 && x <= 122);
}

// 包含数字和.号或-号
fn digital_dot_minus(x: u8, _y: u8) -> bool {
    return (x >= 48 && x <= 57) || x == 46 || x == 45;
}

// 当前是空格，上一个是-或者数字
fn digital_or_none_end(x: u8, y: u8) -> bool {
    return !(x == 32 && ((y >= 48 && y <= 57) || y == 45));
}

// 非空格
fn not_space(x: u8, _y: u8) -> bool {
    return x != 32;
}

struct Line<'a> {
    index: usize,
    origin: &'a str,
    text: &'a [u8],
    len: usize,
}

impl<'a> Line<'a> {
    fn new(text: &'a str) -> Self {
        Self {
            index: 0,
            origin: text,
            text: text.as_bytes(),
            len: text.len(),
        }
    }

    fn parse_item_trim_space<F>(&mut self, cond: F) -> Option<String>
    where
        F: Fn(u8, u8) -> bool,
    {
        let text = self.text;
        let mut i = self.index;
        while i < self.len && text[i] == 32 {
            i += 1;
        }
        self.index = i;
        let mut v = None;
        let mut found_start: i32 = -1;
        let mut found_end: usize = 0;
        let mut y = if i > 0 { text[i - 1] } else { 0 };
        while i < self.len {
            let x = text[i];
            i += 1;
            if cond(x, y) {
                y = x;
                found_end = i - 1;
                if found_start < 0 {
                    found_start = found_end as i32;
                }
                if i < self.len {
                    continue;
                }
            }
            if found_start < 0 {
                // 没有匹配到
                return v;
            }
            v = Some(self.origin[found_start as usize..(found_end + 1)].into());
            while i < self.len && text[i] == 32 {
                i += 1;
            }
            self.index = i;
            return v;
        }
        v
    }

    fn parse_item_wrap_string(&mut self, left: u8, right: u8) -> Option<String> {
        let mut i = self.index;
        while i < self.len && self.text[i] == 32 {
            i += 1;
        }
        if i >= self.len || self.text[i] != left {
            return None;
        }
        i += 1;
        let Some(end) = self.text[i..].iter().position(|&b|b==right) else {
            return None;
        };
        self.index = i + end + 1;
        return Some(self.origin[i..self.index - 1].into());
    }

    fn parse_remote_addr(&mut self) -> Option<String> {
        return self.parse_item_trim_space(digital_dot_colon);
    }

    fn parse_remote_user(&mut self) -> Option<String> {
        let mut i = self.index;
        while i < self.len && self.text[i] == 45 {
            i += 1;
        }
        self.index = i;
        return self.parse_item_trim_space(not_space);
    }

    fn parse_time_local(&mut self) -> Option<String> {
        return self.parse_item_wrap_string(91, 93);
    }

    fn parse_request_line(&mut self) -> Option<String> {
        return self.parse_item_wrap_string(34, 34);
    }

    fn parse_status_code(&mut self) -> Option<String> {
        return self.parse_item_trim_space(digital);
    }

    fn parse_body_bytes_sent(&mut self) -> Option<String> {
        return self.parse_item_trim_space(digital);
    }

    fn parse_http_referer(&mut self) -> Option<String> {
        return self.parse_item_wrap_string(34, 34);
    }

    fn parse_http_user_agent(&mut self) -> Option<String> {
        return self.parse_item_wrap_string(34, 34);
    }

    fn parse_http_x_forwarded_for(&mut self) -> Option<String> {
        return self.parse_item_wrap_string(34, 34);
    }
}

struct LineParser {
    remote_addr_data: HashMap<String, usize>,
    remote_user_data: HashMap<String, usize>,
    time_local_data: HashMap<String, usize>,
    request_line_data: HashMap<String, usize>,
    status_data: HashMap<String, usize>,
    http_referer_data: HashMap<String, usize>,
    http_user_agent_data: HashMap<String, usize>,
    http_x_forwarded_for_data: HashMap<String, usize>,
    http_sent_data: HashMap<String, usize>,
    http_bad_code_data: HashMap<String, HashMap<String, usize>>,
    total_bytes_sent: usize,
    total_lines: usize,
}

impl LineParser {
    fn new() -> Self {
        Self {
            remote_addr_data: HashMap::with_capacity(8192),
            remote_user_data: HashMap::with_capacity(64),
            time_local_data: HashMap::with_capacity(16384),
            request_line_data: HashMap::with_capacity(16384),
            status_data: HashMap::with_capacity(64),
            http_referer_data: HashMap::with_capacity(8192),
            http_user_agent_data: HashMap::with_capacity(8192),
            http_x_forwarded_for_data: HashMap::with_capacity(2048),
            http_sent_data: HashMap::with_capacity(16384),
            http_bad_code_data: HashMap::with_capacity(64),
            total_bytes_sent: 0,
            total_lines: 0,
        }
    }
    fn parse(&mut self, s: &str) -> bool {
        let mut l = Line::new(s);
        let Some(remote_addr)=l.parse_remote_addr()else{return false;};
        let Some(remote_user)=l.parse_remote_user()else{return false;};
        let Some(time_local)=l.parse_time_local()else{return false;};
        let Some(request_line)=l.parse_request_line()else{return false;};
        let Some(status_code)=l.parse_status_code()else{return false;};
        let Some(body_bytes_sent)=l.parse_body_bytes_sent()else{return false;};
        let Some(http_referer)=l.parse_http_referer()else{return false;};
        let Some(http_user_agent)=l.parse_http_user_agent()else{return false;};
        let Some(http_x_forwarded_for)=l.parse_http_x_forwarded_for()else{return false;};

        let body_bytes_sent = body_bytes_sent.parse::<usize>().unwrap();

        self.total_lines += 1;
        self.total_bytes_sent += body_bytes_sent;

        self.remote_addr_data
            .entry(remote_addr)
            .and_modify(|v| *v += 1)
            .or_insert(1);

        self.remote_user_data
            .entry(remote_user)
            .and_modify(|v| *v += 1)
            .or_insert(1);

        self.time_local_data
            .entry(time_local)
            .and_modify(|v| *v += 1)
            .or_insert(1);

        if status_code != "200" {
            self.http_bad_code_data
                .entry(status_code.clone())
                .or_insert_with(|| HashMap::with_capacity(1024))
                .entry(request_line.clone())
                .and_modify(|v| *v += 1)
                .or_insert(1);
        }

        self.http_sent_data
            .entry(request_line.clone())
            .and_modify(|v| *v += body_bytes_sent)
            .or_insert(body_bytes_sent);

        self.request_line_data
            .entry(request_line)
            .and_modify(|v| *v += 1)
            .or_insert(1);

        self.status_data
            .entry(status_code)
            .and_modify(|v| *v += 1)
            .or_insert(1);

        self.http_referer_data
            .entry(http_referer)
            .and_modify(|v| *v += 1)
            .or_insert(1);

        self.http_user_agent_data
            .entry(http_user_agent)
            .and_modify(|v| *v += 1)
            .or_insert(1);

        self.http_x_forwarded_for_data
            .entry(http_x_forwarded_for)
            .and_modify(|v| *v += 1)
            .or_insert(1);

        return true;
    }
}

fn byte_format(n: usize) -> String {
    if n <= 1024 {
        return format!("{} B", n);
    }
    let unit = ["B", "KB", "MB", "GB", "TB", "PB", "EB", "ZB", "YB"];
    let mut pos = 0;
    let mut s = n as f64;
    while s >= 1024.0 {
        s /= 1024.0;
        pos += 1;
    }
    return format!("{:.2} {}", s, unit[pos]);
}

fn sort_by_value<K, V>(map: &HashMap<K, V>) -> Vec<(&K, &V)>
where
    V: std::cmp::Ord,
{
    let mut vec: Vec<(&K, &V)> = map.iter().collect();
    vec.sort_by(|a, b| b.1.cmp(a.1));
    vec
}

fn sort_by_key<K, V>(map: &HashMap<K, V>) -> Vec<(&K, &V)>
where
    K: std::cmp::Ord,
{
    let mut vec: Vec<(&K, &V)> = map.iter().collect();
    vec.sort_by(|a, b| a.0.cmp(b.0));
    vec
}

struct InfoPrinter {
    parser: LineParser,
    limit: usize,
    terminal_width: usize,
}

impl InfoPrinter {
    fn new(parser: LineParser) -> Self {
        let w = get_terminal_width();
        Self {
            parser,
            limit: 100,
            terminal_width: match w {
                20..=9999 => w,
                _ => 100,
            },
        }
    }

    fn print(&self) {
        let ip_count = self.parser.remote_addr_data.len();
        println!("\n共计\x1B[1;34m{}\x1B[00m次访问\n发送总流量\x1B[1;32m{}\x1B[00m\n独立IP数\x1B[1;31m{}\x1B[00m", self.parser.total_lines, byte_format(self.parser.total_bytes_sent), ip_count);
        if self.parser.total_lines < 1 {
            return;
        }
        self.print_stat_long("来访IP统计", &self.parser.remote_addr_data);
        self.print_stat_long("用户统计", &self.parser.remote_user_data);
        self.print_stat_long("代理IP统计", &self.parser.http_x_forwarded_for_data);
        self.print_stat_long("HTTP请求统计", &self.parser.request_line_data);
        self.print_stat_long("User-Agent统计", &self.parser.http_user_agent_data);
        self.print_stat_long("HTTP REFERER 统计", &self.parser.http_referer_data);
        self.print_stat_long("请求时间统计", &self.parser.time_local_data);
        self.print_stat_long("HTTP响应状态统计", &self.parser.status_data);
        self.print_sent_long("HTTP流量占比统计", &self.parser.http_sent_data);

        let http_bad_code_data_sort = sort_by_key(&self.parser.http_bad_code_data);
        for item in http_bad_code_data_sort {
            self.print_code_long(item.0, item.1);
        }
    }

    fn print_stat_long(&self, title: &str, data: &HashMap<String, usize>) {
        println!("\n\x1B[1;34m{}\x1B[00m", title);
        let sorted = sort_by_value(data);
        let mut i = 0;
        let mut n = 0;
        let total_lines = self.parser.total_lines as f64;
        let width = self.terminal_width - 16;
        for item in sorted {
            if i >= self.limit {
                break;
            }
            let x = (100 * item.1) as f64;
            println!(
                "{:<width$.width$} {:6} {:.2}%",
                item.0,
                item.1,
                x / total_lines,
                width = width
            );
            i += 1;
            n += item.1
        }
        let part1 = format!("{}/{}", n, self.parser.total_lines);
        println!(
            "前{}项占比\n{:<width$.width$} {:6} {:.2}%\n",
            self.limit,
            part1,
            data.len(),
            (100 * n) as f64 / total_lines,
            width = width
        )
    }

    fn print_sent_long(&self, title: &str, data: &HashMap<String, usize>) {
        println!("\n\x1B[1;34m{}\x1B[00m", title);
        let sorted = sort_by_value(data);
        let mut i = 0;
        let mut n = 0;
        let total_bytes = self.parser.total_bytes_sent as f64;
        let width = self.terminal_width - 16 - 6;
        for item in sorted {
            if i >= self.limit {
                break;
            }
            let x = (100 * item.1) as f64;
            println!(
                "{:<width$.width$} {:>12} {:.2}%",
                item.0,
                byte_format(*item.1),
                x / total_bytes,
                width = width
            );
            i += 1;
            n += item.1
        }
        let part1 = format!(
            "{}/{}",
            byte_format(n),
            byte_format(self.parser.total_bytes_sent)
        );
        println!(
            "前{}项占比\n{:<width$.width$} {:>12} {:.2}%\n",
            self.limit,
            part1,
            data.len(),
            (100 * n) as f64 / total_bytes,
            width = width
        )
    }

    fn print_code_long(&self, code: &str, data: &HashMap<String, usize>) {
        let sorted = sort_by_value(data);
        let mut count = 0;
        for item in &sorted {
            count += item.1;
        }
        let total_lines = self.parser.total_lines as f64;
        let f_count = count as f64;
        println!(
            "\n\x1B[1;34m状态码{},共{}次,占比{:.2}%\x1B[00m",
            code,
            count,
            (count * 100) as f64 / total_lines
        );
        let mut i = 0;
        let mut n = 0;
        let width = self.terminal_width - 16;
        for item in sorted {
            if i >= self.limit {
                break;
            }
            let x = (100 * item.1) as f64;
            println!(
                "{:<width$.width$} {:6} {:.2}%",
                item.0,
                item.1,
                x / f_count,
                width = width
            );
            i += 1;
            n += item.1
        }
        let part1 = format!("{}/{}", n, f_count);
        println!(
            "前{}项占比\n{:<width$.width$} {:6} {:.2}%\n",
            self.limit,
            part1,
            data.len(),
            (100 * n) as f64 / f_count,
            width = width
        )
    }
}

extern "C" {
    fn ioctl(fd: i32, request: u64, ...) -> i32;
}
fn get_terminal_width() -> usize {
    #[repr(C)]
    struct Winsize {
        ws_row: u16,
        ws_col: u16,
        ws_xpixel: u16,
        ws_ypixel: u16,
    }
    let mut size = Winsize {
        ws_row: 0,
        ws_col: 0,
        ws_xpixel: 0,
        ws_ypixel: 0,
    };
    // 0x40087468 for glibc , 0x005413 for musl
    for s in [0x40087468u64, 0x005413u64] {
        for fd in [0, 1, 2] {
            match unsafe { ioctl(fd, s, &mut size) } {
                0 => break,
                _ => continue,
            }
        }
    }
    size.ws_col as usize
}

fn main() -> std::io::Result<()> {
    let mut parser = LineParser::new();
    let reader: Box<dyn BufRead> = match env::args().nth(1) {
        Some(file) => Box::new(BufReader::new(File::open(file)?)),
        None => Box::new(BufReader::new(stdin())),
    };
    for line in reader.lines() {
        let Ok(a) = line else {
            continue;
        };
        if !parser.parse(&a) {
            eprintln!("{}", &a);
        }
    }
    let printer = InfoPrinter::new(parser);
    printer.print();
    Ok(())
}
