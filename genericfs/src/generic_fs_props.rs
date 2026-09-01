pub trait GenFSProps {
    const FORMAT_NAME: &'static str;

    /// Is detection of this format low confidence, if so it won't be automatically extracted, on CLI you will have to manually request this format
    const LOW_CONFIDENCE_SNIFF: bool = false;
}
