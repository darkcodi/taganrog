use http::{header::*, status::StatusCode};
use http_range::HttpRange;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom, Write};

pub fn get_stream_response(
    path: &str,
    header_map: HeaderMap,
) -> Result<(StatusCode, HeaderMap, Vec<u8>), Box<dyn std::error::Error>> {
    let path = percent_encoding::percent_decode(path.as_bytes())
        .decode_utf8_lossy()
        .to_string();

    let guess = mime_guess::from_path(&path);
    let mime_type = guess.first().map(|mime| mime.to_string()).unwrap_or("application/octet-stream".to_string());

    let mut file = File::open(&path)?;

    let len = {
        let old_pos = file.stream_position()?;
        let len = file.seek(SeekFrom::End(0))?;
        file.seek(SeekFrom::Start(old_pos))?;
        len
    };

    let mut response_code = StatusCode::OK;

    let mut response_headers = HeaderMap::new();
    response_headers.insert(CONTENT_TYPE, HeaderValue::from_str(&mime_type)?);
    response_headers.insert(ACCESS_CONTROL_ALLOW_ORIGIN, HeaderValue::from_str("*")?);

    // if the webview sent a range header, we need to send a 206 in return
    if let Some(range_header) = header_map.get("range") {
        // parse range header
        let ranges = if let Ok(ranges) = HttpRange::parse(range_header.to_str()?, len) {
            ranges
                .iter()
                // map the output back to spec range <start-end>, example: 0-499
                .map(|r| (r.start, r.start + r.length - 1))
                .collect::<Vec<_>>()
        } else {
            response_headers.insert(CONTENT_RANGE, format!("bytes */{len}").parse()?);
            return Ok((StatusCode::RANGE_NOT_SATISFIABLE, response_headers, Vec::new()));
        };

        /// The Maximum bytes we send in one range
        const MAX_LEN: u64 = 1000 * 1024;

        if ranges.len() == 1 {
            let &(start, mut end) = ranges.first().unwrap();

            // check if a range is not satisfiable
            //
            // this should be already taken care of by HttpRange::parse
            // but checking here again for extra assurance
            if start >= len || end >= len || end < start {
                response_headers.insert(CONTENT_RANGE, format!("bytes */{len}").parse()?);
                return Ok((StatusCode::RANGE_NOT_SATISFIABLE, response_headers, Vec::new()));
            }

            // adjust end byte for MAX_LEN
            end = start + (end - start).min(len - start).min(MAX_LEN - 1);

            // calculate number of bytes needed to be read
            let bytes_to_read = end + 1 - start;

            // allocate a buf with a suitable capacity
            let mut buf = Vec::with_capacity(bytes_to_read as usize);
            // seek the file to the starting byte
            file.seek(SeekFrom::Start(start))?;
            // read the needed bytes
            file.take(bytes_to_read).read_to_end(&mut buf)?;

            response_headers.insert(CONTENT_RANGE, format!("bytes {start}-{end}/{len}").parse()?);
            response_headers.insert(CONTENT_LENGTH, HeaderValue::from(end + 1 - start));
            response_code = StatusCode::PARTIAL_CONTENT;

            Ok((response_code, response_headers, buf))
        } else {
            let mut buf = Vec::new();
            let ranges = ranges
                .iter()
                .filter_map(|&(start, mut end)| {
                    // filter out unsatisfiable ranges
                    //
                    // this should be already taken care of by HttpRange::parse
                    // but checking here again for extra assurance
                    if start >= len || end >= len || end < start {
                        None
                    } else {
                        // adjust end byte for MAX_LEN
                        end = start + (end - start).min(len - start).min(MAX_LEN - 1);
                        Some((start, end))
                    }
                })
                .collect::<Vec<_>>();

            let boundary = random_boundary();
            let boundary_sep = format!("\r\n--{boundary}\r\n");
            let boundary_closer = format!("\r\n--{boundary}\r\n");

            response_headers.insert(CONTENT_TYPE, format!("multipart/byteranges; boundary={boundary}").parse()?);

            for (end, start) in ranges {
                // a new range is being written, write the range boundary
                buf.write_all(boundary_sep.as_bytes())?;

                // write the needed headers `Content-Type` and `Content-Range`
                buf.write_all(format!("{CONTENT_TYPE}: video/mp4\r\n").as_bytes())?;
                buf.write_all(format!("{CONTENT_RANGE}: bytes {start}-{end}/{len}\r\n").as_bytes())?;

                // write the separator to indicate the start of the range body
                buf.write_all("\r\n".as_bytes())?;

                // calculate number of bytes needed to be read
                let bytes_to_read = end + 1 - start;

                let mut local_buf = vec![0_u8; bytes_to_read as usize];
                file.seek(SeekFrom::Start(start))?;
                file.read_exact(&mut local_buf)?;
                buf.extend_from_slice(&local_buf);
            }
            // all ranges have been written, write the closing boundary
            buf.write_all(boundary_closer.as_bytes())?;

            Ok((response_code, response_headers, buf))
        }
    } else {
        response_headers.insert(CONTENT_LENGTH, HeaderValue::from(len));

        let mut buf = Vec::with_capacity(len as usize);
        file.read_to_end(&mut buf)?;

        Ok((response_code, response_headers, buf))
    }
}

fn random_boundary() -> String {
    let mut x = [0_u8; 30];
    getrandom::getrandom(&mut x).expect("failed to get random bytes");
    (x[..])
        .iter()
        .map(|&x| format!("{x:x}"))
        .fold(String::new(), |mut a, x| {
            a.push_str(x.as_str());
            a
        })
}
