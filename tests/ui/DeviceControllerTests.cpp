#include <QtTest>
#include "DeviceCenterController.h"
#include "sony/core/DeviceService.h"
#include "../support/ReplyTransport.h"
#include <chrono>
using namespace sony;
using namespace sony::devicecenter;
class SlowService : public core::IDeviceService {
public:
    void tick() override { std::this_thread::sleep_for(std::chrono::milliseconds(200)); }
    std::vector<core::DiscoveredDevice> discoverDevices() override { return {}; }
    void connect(const transport::DeviceAddress&, std::string_view) override {}
    void disconnect() noexcept override {}
    bool isConnected() const noexcept override { return false; }
    core::SonyDevice* activeDevice() noexcept override { return nullptr; }
    protocol::DeviceStateSnapshot snapshot() const override { return std::make_shared<const protocol::DeviceState>(); }
};
class DeviceControllerTests : public QObject {
    Q_OBJECT
private slots:
    void headsetConnectedDevicesFollowNotificationsAndDisconnect() {
        auto transport = std::make_shared<ReplyTransport>();
        transport->peripheralSupported = true;
        auto service = std::make_shared<core::DeviceService>(transport);
        service->connect(transport::DeviceAddress("11:22:33:44:55:66"), "ULT WEAR");
        DeviceCenterController controller(nullptr, service);
        QTRY_VERIFY_WITH_TIMEOUT(!controller.busy(), 2000);
        QVERIFY(controller.hasConnectedDevices());
        QCOMPARE(controller.connectedDevicesStatus(), QString("valid"));
        QCOMPARE(controller.connectedDevices().size(), 2);
        QCOMPARE(controller.connectedDevices()[0].toMap()["name"].toString(), QString::fromUtf8("Ayana\xe2\x80\x99s iPhone"));
        QVERIFY(controller.connectedDevices()[1].toMap()["playbackDevice"].toBool());
        // Same opcode on table 1 must not overwrite the headset list.
        transport->notify({0x39,2,0,0});
        QTest::qWait(50);
        QCOMPARE(controller.connectedDevices().size(), 2);
        transport->notify({0x39,2,0,0}, protocol::DataType::DataMdrNo2);
        QTRY_VERIFY_WITH_TIMEOUT(controller.connectedDevices().isEmpty(), 1000);
        QCOMPARE(controller.connectedDevicesStatus(), QString("valid"));
        transport->notify(ReplyTransport::peripheralList(0x39), protocol::DataType::DataMdrNo2);
        QTRY_COMPARE_WITH_TIMEOUT(controller.connectedDevices().size(), 2, 1000);
        // A malformed report cannot leave old names labelled as live connections.
        transport->notify({0x39,2,2}, protocol::DataType::DataMdrNo2);
        QTRY_COMPARE_WITH_TIMEOUT(controller.connectedDevicesStatus(), QString("stale"), 1000);
        QVERIFY(controller.connectedDevices().isEmpty());
        transport->notify(ReplyTransport::peripheralList(0x39), protocol::DataType::DataMdrNo2);
        QTRY_COMPARE_WITH_TIMEOUT(controller.connectedDevices().size(), 2, 1000);
        controller.disconnectDevice();
        QTRY_VERIFY_WITH_TIMEOUT(!controller.busy(), 2000);
        QVERIFY(controller.connectedDevices().isEmpty());
    }
    void headsetDeviceListBecomesAvailableWhenMultipointSupportChanges() {
        auto transport = std::make_shared<ReplyTransport>();
        auto service = std::make_shared<core::DeviceService>(transport);
        service->connect(transport::DeviceAddress("11:22:33:44:55:66"), "ULT WEAR");
        DeviceCenterController controller(nullptr, service);
        QTRY_VERIFY_WITH_TIMEOUT(!controller.busy(), 2000);
        QCOMPARE(controller.connectedDevicesStatus(), QString("unsupported"));
        QVERIFY(controller.connectedDevices().isEmpty());
        transport->peripheralSupported = true;
        // Notification models the newly advertised list; refresh also re-inquires support.
        transport->notify(ReplyTransport::peripheralList(0x39), protocol::DataType::DataMdrNo2);
        QTRY_COMPARE_WITH_TIMEOUT(controller.connectedDevicesStatus(), QString("valid"), 1000);
        QCOMPARE(controller.connectedDevices().size(), 2);
    }
    void startupDoesNotBlockGui() {
        auto service = std::make_shared<SlowService>();
        QElapsedTimer elapsed; elapsed.start();
        DeviceCenterController controller(nullptr, service);
        QVERIFY(elapsed.elapsed() < 100);
        QVERIFY(controller.busy());
        bool guiTick = false;
        QTimer::singleShot(10, [&] { guiTick = true; });
        QTRY_VERIFY_WITH_TIMEOUT(guiTick, 100);
        QTRY_VERIFY_WITH_TIMEOUT(!controller.busy(), 2000);
        QCOMPARE(controller.batteryLevel(), -1);
        QCOMPARE(controller.noiseControlMode(), QString("unknown"));
        QCOMPARE(controller.codec(), QString("Unknown"));
    }
    void equalizerCapabilities_data() {
        QTest::addColumn<QString>("model");
        QTest::addColumn<bool>("tenBand");
        QTest::newRow("WH-1000XM6") << QString("WH-1000XM6") << true;
        QTest::newRow("WH-1000XM5") << QString("WH-1000XM5") << false;
    }
    void equalizerCapabilities() {
        QFETCH(QString, model);
        QFETCH(bool, tenBand);
        auto transport = std::make_shared<ReplyTransport>();
        auto service = std::make_shared<core::DeviceService>(transport);
        service->connect(transport::DeviceAddress("11:22:33:44:55:66"), model.toStdString());
        DeviceCenterController controller(nullptr, service);
        QSignalSpy capabilitiesChanged(&controller, &DeviceCenterController::capabilitiesChanged);
        QTRY_VERIFY_WITH_TIMEOUT(!controller.busy(), 2000);
        QVERIFY(!capabilitiesChanged.isEmpty());
        QVERIFY(controller.hasEqualizer());
        QCOMPARE(controller.tenBandEqualizer(), tenBand);
        QCOMPARE(controller.property("tenBandEqualizer"), QVariant(tenBand));
        QCOMPARE(controller.hasClearBass(), !tenBand);
    }
    void failedActionPreservesConfirmedValue() {
        auto transport = std::make_shared<ReplyTransport>();
        auto service = std::make_shared<core::DeviceService>(transport);
        service->connect(transport::DeviceAddress("11:22:33:44:55:66"),"WH-1000XM5");
        DeviceCenterController controller(nullptr, service);
        QTRY_VERIFY_WITH_TIMEOUT(!controller.busy(),2000);
        QCOMPARE(controller.noiseControlMode(),QString("cancelling"));
        transport->simulateTimeoutOnSend();
        controller.setNoiseControlOff();
        QVERIFY(controller.busy());
        QCOMPARE(controller.noiseControlMode(),QString("cancelling"));
        QTRY_VERIFY_WITH_TIMEOUT(!controller.busy(),3000);
        QVERIFY(!controller.lastError().isEmpty());
        QCOMPARE(controller.noiseControlMode(),QString("cancelling"));
    }
    void notificationsUpdateStateAndBands() {
        auto transport = std::make_shared<ReplyTransport>();
        auto service = std::make_shared<core::DeviceService>(transport);
        service->connect(transport::DeviceAddress("11:22:33:44:55:66"),"WH-1000XM5");
        DeviceCenterController controller(nullptr, service);
        QTRY_VERIFY_WITH_TIMEOUT(!controller.busy(),2000);
        QCOMPARE(controller.equalizerBands().size(),5);
        QCOMPARE(controller.equalizerBands()[4].toInt(),5);
        transport->notify({0x69,0x17,1,1,1,0,12});
        QTRY_COMPARE_WITH_TIMEOUT(controller.noiseControlMode(),QString("ambient"),1000);
        QCOMPARE(controller.ambientLevel(),12);
    }
    void destructionDrainsWorkerAndCallbacks() {
        auto service = std::make_shared<SlowService>();
        auto controller = std::make_unique<DeviceCenterController>(nullptr,service);
        QTest::qWait(20);
        controller.reset();
        QCoreApplication::processEvents();
    }
    void ambientModeKeepsAValidLevelAcrossAncAndOff() {
        auto transport = std::make_shared<ReplyTransport>();
        auto service = std::make_shared<core::DeviceService>(transport);
        service->connect(transport::DeviceAddress("11:22:33:44:55:66"), "ULT WEAR");
        DeviceCenterController controller(nullptr, service);
        QTRY_VERIFY_WITH_TIMEOUT(!controller.busy(), 2000);
        QCOMPARE(controller.noiseControlMode(), QString("cancelling"));
        QCOMPARE(controller.ambientLevel(), 10);

        controller.setAmbient(controller.ambientLevel(), false);
        QTRY_VERIFY_WITH_TIMEOUT(!controller.busy(), 2000);
        QVERIFY(controller.lastError().isEmpty());
        QCOMPARE(controller.noiseControlMode(), QString("ambient"));

        transport->notify({0x69,0x17,1,1,1,0,12});
        QTRY_COMPARE_WITH_TIMEOUT(controller.ambientLevel(), 12, 1000);
        transport->notify({0x69,0x17,1,1,0,0,0});
        QTRY_COMPARE_WITH_TIMEOUT(controller.noiseControlMode(), QString("cancelling"), 1000);
        QCOMPARE(controller.ambientLevel(), 12);

        controller.setNoiseControlOff();
        QTRY_VERIFY_WITH_TIMEOUT(!controller.busy(), 2000);
        QCOMPARE(controller.noiseControlMode(), QString("off"));
        QCOMPARE(controller.ambientLevel(), 12);
        controller.setAmbient(0, false);
        QTRY_VERIFY_WITH_TIMEOUT(!controller.busy(), 2000);
        QVERIFY(controller.lastError().isEmpty());
        QCOMPARE(controller.noiseControlMode(), QString("ambient"));
        QCOMPARE(controller.ambientLevel(), 12);
    }
    void pauseOnRemovalUsesConfirmedHeadsetState() {
        auto transport = std::make_shared<ReplyTransport>();
        auto service = std::make_shared<core::DeviceService>(transport);
        service->connect(transport::DeviceAddress("11:22:33:44:55:66"), "ULT WEAR");
        DeviceCenterController controller(nullptr, service);
        QTRY_VERIFY_WITH_TIMEOUT(!controller.busy(), 2000);
        QVERIFY(controller.hasPauseWhenRemoved());
        QVERIFY(controller.pauseWhenRemoved());
        controller.setPauseWhenRemoved(false);
        QVERIFY(controller.pauseWhenRemoved());
        QTRY_VERIFY_WITH_TIMEOUT(!controller.busy(), 2000);
        QVERIFY(!controller.pauseWhenRemoved());
        controller.setPauseWhenRemoved(true);
        QTRY_VERIFY_WITH_TIMEOUT(!controller.busy(), 2000);
        QVERIFY(controller.pauseWhenRemoved());
        QCOMPARE(controller.property("pauseWhenRemoved"), QVariant(true));
    }
    void rejectedPauseToggleKeepsConfirmedValue() {
        auto transport = std::make_shared<ReplyTransport>();
        transport->rejectPauseChanges = true;
        auto service = std::make_shared<core::DeviceService>(transport);
        service->connect(transport::DeviceAddress("11:22:33:44:55:66"), "ULT WEAR");
        DeviceCenterController controller(nullptr, service);
        QTRY_VERIFY_WITH_TIMEOUT(!controller.busy(), 2000);
        controller.setPauseWhenRemoved(false);
        QTRY_VERIFY_WITH_TIMEOUT(!controller.busy(), 3000);
        QVERIFY(controller.pauseWhenRemoved());
        QVERIFY(!controller.lastError().isEmpty());
    }
    void ultButtonUpdatesModeWithoutChangingEq() {
        auto transport = std::make_shared<ReplyTransport>();
        auto service = std::make_shared<core::DeviceService>(transport);
        service->connect(transport::DeviceAddress("11:22:33:44:55:66"), "ULT WEAR");
        DeviceCenterController controller(nullptr, service);
        QTRY_VERIFY_WITH_TIMEOUT(!controller.busy(), 2000);
        QVERIFY(controller.hasUlt());
        QCOMPARE(controller.ultMode(), 0);
        const auto bands = controller.equalizerBands();
        for (int mode : {1, 2, 0}) {
            transport->ultMode = mode;
            transport->notify({0x59,3,0xa0,static_cast<uint8_t>(mode),6,10,11,12,13,14,15});
            QTRY_COMPARE_WITH_TIMEOUT(controller.ultMode(), mode, 1000);
            QCOMPARE(controller.equalizerBands(), bands);
            QCOMPARE(controller.equalizerPresetName(), QString("Custom"));
        }
        controller.disconnectDevice();
        QTRY_VERIFY_WITH_TIMEOUT(!controller.busy(), 2000);
        QCOMPARE(controller.ultMode(), -1);
    }
};
QTEST_GUILESS_MAIN(DeviceControllerTests)
#include "DeviceControllerTests.moc"
