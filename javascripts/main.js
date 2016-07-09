var app = angular.module('MainApp', ['ui.router']);

app.config(['$stateProvider', '$urlRouterProvider', function ($stateProvider, $urlRouterProvider) {

  $urlRouterProvider.otherwise('/');

  $stateProvider.state('home', {
    url:'/',
    templateUrl: 'views/homepage.html'
  });

}]);