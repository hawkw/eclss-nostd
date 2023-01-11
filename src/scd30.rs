use crate::I2cRef;
use esp32c3_hal::{
    clock::Clocks,
    i2c::{self, I2C},
    pac::I2C0,
    Delay,
};
use esp_println::println;

pub type Sensor<'bus> = sensor_scd30::Scd30<I2cRef<'bus>, Delay, i2c::Error>;
pub type Error = sensor_scd30::Error<i2c::Error>;

pub fn bringup<'bus>(
    busman: &'bus shared_bus::BusManagerSimple<I2C<I2C0>>,
    clocks: &Clocks,
) -> Result<Sensor<'bus>, Error> {
    println!("connecting to SCD30");
    let mut scd30 = retry(10, || {
        let i2c = busman.acquire_i2c();
        let delay = Delay::new(clocks);
        sensor_scd30::Scd30::new(i2c, delay)
    })
    .expect("failed to initialize SCD30");

    let firmware =
        retry(10, || scd30.firmware_version()).expect("failed to read SCD30 firmware version");
    println!("connected to SCD30; firmware: {firmware}");
    // retry(10, || scd30.set_afc(false)).expect("failed to enable automatic calibration mode");
    // println!("enabled SCD30 automatic calibration");
    // Start continuous sampling mode
    retry(10, || scd30.start_continuous(10)).expect("failed to start continuous mode"); // TODO(eliza): figure out pressure compensation.
    println!("enabled SCD30 continuous sampling mode");
    Ok(scd30)
}

fn retry<T>(mut retries: usize, mut f: impl FnMut() -> Result<T, Error>) -> Result<T, Error> {
    loop {
        match f() {
            Ok(val) => return Ok(val),
            Err(sensor_scd30::Error::NoDevice) => return Err(sensor_scd30::Error::NoDevice),
            Err(error) if retries == 0 => return Err(error),
            Err(error) => {
                retries -= 1;
                println!("[SCD30] retrying: {error:?} ({retries} retries remaining)");
            }
        }
    }
}
