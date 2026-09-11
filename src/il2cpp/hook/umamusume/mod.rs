pub mod Localize;
pub mod TextId;
pub mod StoryRaceTextAsset;
mod LyricsController;
pub mod StoryTimelineData;
pub mod StoryTimelineBlockData;
pub mod StoryTimelineTrackData;
pub mod StoryTimelineTextClipData;
pub mod GallopUtil;
pub mod UIManager;
pub mod GraphicSettings;
mod CameraController;
pub mod SingleModeStartResultCharaViewer;
pub mod WebViewManager;
pub mod DialogCommon;
mod PartsSingleModeSkillLearningListItem;
mod MasterMissionData;
mod TrainingParamChangeA2U;
pub mod WebViewDefine;
pub mod TextFrame;
pub mod PartsSingleModeSkillListItem;
pub mod FlashActionPlayer;
pub mod TextRubyData;
pub mod TextDotData;
pub mod GameSystem;
pub mod StoryViewTextControllerLandscape;
pub mod StoryViewTextControllerSingleMode;
mod JikkyoDisplay;
pub mod Screen;
mod TrainingParamChangePlate;
mod SingleModeUtils;
mod MasterSingleModeTurn;
mod TextFontManager;
mod TextFormat;
pub mod TextCommon;
mod TextMeshProUguiCommon;
mod StoryChoiceController;
mod StoryViewController;
mod StoryTimelineClipData;
mod StoryTimelineCharaTrackData;
mod CharacterNoteTopView;
mod CharacterNoteTopViewController;
mod ViewControllerBase;
mod ButtonCommon;
mod NowLoading;
pub mod StoryTimelineController;
mod DialogRaceOrientation;
mod RaceInfo;
mod RaceUtil;
mod SaveDataManager;
mod ApplicationSettingSaveLoader;
mod LiveTheaterCharaSelect;
mod LiveTheaterViewController;
pub mod CySpringController;
mod LiveUtil;
pub mod MasterDataUtil;
pub mod DialogCommonBase;
pub mod DialogObject;
pub mod ImageCommon;
pub mod TimeUtil;
pub mod CameraData;
pub mod DialogManager;
mod CharacterHomeTopUI;
mod CharacterHomeTopUIController;
mod PartsGachaButton;
mod PartsHorizontalTextSet;
mod GachaExecutableUnit;
mod StoryChoiceButton;
pub mod PartsSingleModeStoryEventTitle;

pub mod SceneManager;

#[cfg(target_os = "windows")]
mod PaymentUtility;
mod LowResolutionCamera;
pub mod TweenAnimationTimelineComponent;
pub mod TweenAnimationTimelineData;
pub mod TweenAnimationTimelineSheetData;
mod PartsSingleModeChoiceRewardTextElementViewModel;
mod PartsCommonHeaderTitle;
pub mod PartsCharaMessageBase;
pub mod StoryParamChangeEffect;
pub mod DialogSupportCardDetail;
pub mod StoryTimelineLiveStreamingClipData;
pub mod LiveStreamingCommentScriptableObject;
mod Connecting;
mod DownloadManager;
mod DownloadView;
mod HttpHelper;
mod DownloadErrorProcessor;
mod TitleViewController;
pub mod Director;
mod CySpringNative;
mod CySpringUpdater;
mod PartsRaceAnalyzeRaceEventListItem;

pub mod AudioManager;

// Ported from kairusds/Hachimi-Edge (additive merge)
pub mod StoryViewTextControllerBase;
#[cfg(target_os = "windows")]
pub mod LandscapeUIManager;
#[cfg(target_os = "windows")]
pub mod StandaloneWindowResize;
#[cfg(target_os = "windows")]
mod GallopInput;
#[cfg(target_os = "windows")]
mod InputSystemManager;
#[cfg(target_os = "windows")]
mod BackKeyInputManager;
#[cfg(target_os = "windows")]
pub mod WindowsGamepadControl;
pub mod TapEffectController;
pub mod MasterCharacterSystemText;
pub mod Notification;
#[cfg(target_os = "windows")]
mod LiveTimelineControl;
#[cfg(target_os = "windows")]
pub mod LiveTimelineWorkSheet;
#[cfg(target_os = "windows")]
pub mod LiveTimelineKeyPostFilmDataList;
#[cfg(target_os = "windows")]
pub mod LiveTimelineKeyCameraPositionData;
#[cfg(target_os = "windows")]
mod LiveTimelineKeyCameraLookAtData;
#[cfg(target_os = "windows")]
mod LiveTimelineKeyMultiCameraPositionData;
#[cfg(target_os = "windows")]
mod CharacterObject;
#[cfg(target_os = "windows")]
mod LiveModelController;
#[cfg(target_os = "windows")]
pub mod ModelController;
#[cfg(target_os = "windows")]
mod RaceCameraManager;
#[cfg(target_os = "windows")]
mod RaceCameraEventBase;
#[cfg(target_os = "windows")]
mod RaceModelController;
#[cfg(target_os = "windows")]
mod RaceViewBase;
#[cfg(target_os = "windows")]
mod RaceEffectManager;
pub mod HorseData;
pub mod HorseRaceInfo;
pub mod RaceHorseData;
pub mod HorseRaceAIBase;
pub mod HorseRaceAIReplay;
#[cfg(target_os = "windows")]
mod HorseRaceInfoReplay;
pub mod JikkyoControllerBase;
pub mod Jikkyo;
pub mod RaceBGMController;
pub mod RaceMainViewController;
pub mod RaceManager;
pub mod RaceManagerReplayBase;
pub mod RaceEventPlayer;
pub mod RaceHorseManagerBase;
pub mod RaceHorseManagerReplay;
pub mod RaceSoundReplay;
pub mod RaceUI;
pub mod RaceUIMiniMap;
pub mod RaceViewReplay;
pub mod RaceSimulateData;
pub mod RaceSimulateHorseResultData;
pub mod RaceSimulateEventData;
pub mod RaceSimulateReader;
pub mod SimulateEventType;
pub mod SkillManager;
#[cfg(target_os = "windows")]
mod PartsScheduleBookAutoPlayScreen;
pub mod PartsNickNameRibbon;
mod PartsNickNameListItem;
mod PartsGetSkillPlate;
mod DialogMissionListItem;
mod PartsNamePlateBase;
mod PartsSupportCardImproveDetail;
#[cfg(target_os = "windows")]
pub mod MainGameInitializer;
pub mod LiveViewController;
pub mod LiveTimeController;
pub mod HomeViewController;
pub mod WorkDataManager;
pub mod AssetManager;
pub mod WorkJukeboxData;
pub mod JukeboxBgmSelector;
pub mod JukeboxHomeTopUI;
pub mod TempData;
pub mod MasterJukeboxSetlistMusicData;
pub mod HubViewControllerBase;
mod LiveTheaterInfo;
pub mod DownloadPathRegister;
pub mod SceneDefine;
pub mod GameDefine;
pub mod MasterDataManager;
pub mod MasterItemExchangeTop;
pub mod MasterSkillData;
pub mod race_telemetry;

pub fn init() {
    get_assembly_image_or_return!(image, "umamusume.dll");

    Localize::init(image);
    TextId::init(image);
    StoryRaceTextAsset::init(image);
    LyricsController::init(image);
    StoryTimelineData::init(image);
    StoryTimelineBlockData::init(image);
    StoryTimelineTrackData::init(image);
    StoryTimelineTextClipData::init(image);
    GallopUtil::init(image);
    UIManager::init(image);
    GraphicSettings::init(image);
    CameraController::init(image);
    SingleModeStartResultCharaViewer::init(image);
    WebViewManager::init(image);
    DialogCommon::init(image);
    PartsSingleModeSkillLearningListItem::init(image);
    MasterMissionData::init(image);
    TrainingParamChangeA2U::init(image);
    TextFrame::init(image);
    PartsSingleModeSkillListItem::init(image);
    FlashActionPlayer::init(image);
    TextRubyData::init(image);
    TextDotData::init(image);
    GameSystem::init(image);
    StoryViewTextControllerLandscape::init(image);
    StoryViewTextControllerSingleMode::init(image);
    JikkyoDisplay::init(image);
    Screen::init(image);
    TrainingParamChangePlate::init(image);
    SingleModeUtils::init(image);
    MasterSingleModeTurn::init(image);
    TextFontManager::init(image);
    TextFormat::init(image);
    TextCommon::init(image);
    TextMeshProUguiCommon::init(image);
    StoryChoiceController::init(image);
    StoryViewController::init(image);
    StoryTimelineClipData::init(image);
    StoryTimelineLiveStreamingClipData::init(image);
    StoryTimelineCharaTrackData::init(image);
    CharacterNoteTopView::init(image);
    CharacterNoteTopViewController::init(image);
    ViewControllerBase::init(image);
    ButtonCommon::init(image);
    NowLoading::init(image);
    StoryTimelineController::init(image);
    DialogRaceOrientation::init(image);
    RaceInfo::init(image);
    RaceUtil::init(image);
    SaveDataManager::init(image);
    ApplicationSettingSaveLoader::init(image);
    LiveTheaterCharaSelect::init(image);
    LiveTheaterViewController::init(image);
    CySpringController::init(image);
    LiveUtil::init(image);
    MasterDataUtil::init(image);
    DialogCommonBase::init(image);
    DialogObject::init(image);
    ImageCommon::init(image);
    TimeUtil::init(image);
    DialogManager::init(image);
    CharacterHomeTopUI::init(image);
    CharacterHomeTopUIController::init(image);
    PartsGachaButton::init(image);
    PartsHorizontalTextSet::init(image);
    GachaExecutableUnit::init(image);
    StoryChoiceButton::init(image);
    PartsSingleModeStoryEventTitle::init(image);

    SceneManager::init(image);
    #[cfg(target_os = "windows")]
    {
        PaymentUtility::init(image);
    }
    LowResolutionCamera::init(image);
    CameraData::init(image);
    TweenAnimationTimelineComponent::init(image);
    TweenAnimationTimelineData::init(image);
    TweenAnimationTimelineSheetData::init(image);
    PartsSingleModeChoiceRewardTextElementViewModel::init(image);
    PartsCommonHeaderTitle::init(image);
    PartsCharaMessageBase::init(image);
    StoryParamChangeEffect::init(image);
    DialogSupportCardDetail::init(image);
    Connecting::init(image);
    DownloadManager::init(image);
    DownloadView::init(image);
    HttpHelper::init(image);
    DownloadErrorProcessor::init(image);
    TitleViewController::init(image);
    Director::init(image);
    CySpringNative::init(image);
    CySpringUpdater::init(image);
    PartsRaceAnalyzeRaceEventListItem::init(image);
    AudioManager::init(image);
    LiveStreamingCommentScriptableObject::init(image);
    TapEffectController::init(image);

    // Ported from kairusds/Hachimi-Edge (additive merge)
    StoryViewTextControllerBase::init(image);
    #[cfg(target_os = "windows")]
    {
        StandaloneWindowResize::init(image);
        GallopInput::init(image);
        InputSystemManager::init(image);
        BackKeyInputManager::init(image);
        WindowsGamepadControl::init(image);
        MainGameInitializer::init(image);
        LiveTimelineControl::init(image);
        LiveTimelineWorkSheet::init(image);
        LiveTimelineKeyPostFilmDataList::init(image);
        LiveTimelineKeyCameraPositionData::init(image);
        LiveTimelineKeyCameraLookAtData::init(image);
        LiveTimelineKeyMultiCameraPositionData::init(image);
        CharacterObject::init(image);
        LiveModelController::init(image);
        ModelController::init(image);
        RaceCameraManager::init(image);
        RaceCameraEventBase::init(image);
        RaceModelController::init(image);
        RaceViewBase::init(image);
        RaceEffectManager::init(image);
        HorseRaceInfoReplay::init(image);
        PartsScheduleBookAutoPlayScreen::init(image);
        LandscapeUIManager::init(image);
    }
    HorseData::init(image);
    HorseRaceInfo::init(image);
    RaceHorseData::init(image);
    HorseRaceAIBase::init(image);
    HorseRaceAIReplay::init(image);
    JikkyoControllerBase::init(image);
    Jikkyo::init(image);
    RaceBGMController::init(image);
    RaceMainViewController::init(image);
    RaceManager::init(image);
    RaceManagerReplayBase::init(image);
    RaceEventPlayer::init(image);
    RaceSoundReplay::init(image);
    RaceUI::init(image);
    RaceUIMiniMap::init(image);
    RaceViewReplay::init(image);
    RaceHorseManagerBase::init(image);
    RaceHorseManagerReplay::init(image);
    RaceSimulateData::init(image);
    RaceSimulateHorseResultData::init(image);
    RaceSimulateEventData::init(image);
    RaceSimulateReader::init(image);
    SkillManager::init(image);
    MasterCharacterSystemText::init(image);
    Notification::init(image);
    PartsNickNameRibbon::init(image);
    PartsNickNameListItem::init(image);
    PartsGetSkillPlate::init(image);
    DialogMissionListItem::init(image);
    PartsNamePlateBase::init(image);
    PartsSupportCardImproveDetail::init(image);
    LiveViewController::init(image);
    LiveTimeController::init(image);
    HomeViewController::init(image);
    WorkDataManager::init(image);
    AssetManager::init(image);
    WorkJukeboxData::init(image);
    JukeboxBgmSelector::init(image);
    JukeboxHomeTopUI::init(image);
    TempData::init(image);
    MasterJukeboxSetlistMusicData::init(image);
    HubViewControllerBase::init(image);
    LiveTheaterInfo::init(image);
    DownloadPathRegister::init(image);
    SceneDefine::init(image);
    GameDefine::init(image);
    MasterDataManager::init(image);
    MasterItemExchangeTop::init(image);
    MasterSkillData::init(image);
}