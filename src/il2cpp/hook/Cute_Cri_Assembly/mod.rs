mod MovieManager;
pub mod AudioControllerBase;
pub mod AtomSourceEx;
pub mod AudioManager;
pub mod AudioPlayback;
pub mod CuteAudioSource;
pub mod CuteAudioSourcePool;

pub fn init() {
    get_assembly_image_or_return!(image, "Cute.Cri.Assembly.dll");

    MovieManager::init(image);
    AudioControllerBase::init(image);
    AtomSourceEx::init(image);
    AudioManager::init(image);
    AudioPlayback::init(image);
    CuteAudioSource::init(image);
    CuteAudioSourcePool::init(image);
}
