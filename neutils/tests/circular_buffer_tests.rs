
use neutils::circular_buffer::CircularBuffer; // Replace `your_crate_name` with the name of your crate

#[test]
fn test_push_back_pop_front() {
    let mut buffer = CircularBuffer::new(5);

    buffer.push_back(1);
    buffer.push_back(2);
    buffer.push_back(3);

    assert_eq!(buffer.pop_front(), Some(1));
    assert_eq!(buffer.pop_front(), Some(2));

    buffer.push_back(4);
    buffer.push_back(5);

    assert_eq!(buffer.pop_front(), Some(3));
    assert_eq!(buffer.pop_front(), Some(4));
    assert_eq!(buffer.pop_front(), Some(5));
    assert_eq!(buffer.pop_front(), None);
}

#[test]
fn test_overflow() {
    let mut buffer = CircularBuffer::new(3);

    buffer.push_back(1);
    buffer.push_back(2);
    buffer.push_back(3);
    buffer.push_back(4); // This will cause an overflow

    assert_eq!(buffer.pop_front(), Some(2));
    assert_eq!(buffer.pop_front(), Some(3));
    assert_eq!(buffer.pop_front(), Some(4));
    assert_eq!(buffer.pop_front(), None);
}


#[test]
fn test_is_empty() {
    let mut buffer = CircularBuffer::new(5);

    assert!(buffer.is_empty());

    buffer.push_back(1);
    buffer.push_back(2);

    assert!(!buffer.is_empty());

    buffer.pop_front();
    buffer.pop_front();

    assert!(buffer.is_empty());
}

#[test]
fn test_extend() {
    let mut buffer = CircularBuffer::new(5);

    buffer.extend(&[1, 2, 3]);

    assert_eq!(buffer.pop_front(), Some(1));
    assert_eq!(buffer.pop_front(), Some(2));
    assert_eq!(buffer.pop_front(), Some(3));
    assert_eq!(buffer.pop_front(), None);
}
